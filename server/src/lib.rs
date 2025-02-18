use axum::{
	error_handling::HandleErrorLayer,
	extract::{Request, State},
	http::{header, Method, StatusCode, Uri},
	middleware::{self, Next},
	response::Response,
	BoxError, Router,
};
use eyre::{Report, Result};
use signal::unix::SignalKind;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{trace, trace::TraceLayer, LatencyUnit};
use tracing::{info, warn, Level};

use crate::errors::ServerError;
use barreleye_common::{models::ApiKey, quit, App, AppError};

mod errors;
mod handlers;
mod utils;

pub type ServerResult<'a, T> = Result<T, ServerError<'a>>;

pub struct Server {
	app: Arc<App>,
}

impl Server {
	pub fn new(app: Arc<App>) -> Self {
		Self { app }
	}

	async fn auth(
		State(app): State<Arc<App>>,
		req: Request,
		next: Next,
	) -> ServerResult<'static, Response> {
		if ApiKey::count(app.db()).await? == 0 {
			return Ok(next.run(req).await);
		}

		for public_endpoint in ["/v1/info"].iter() {
			if req.uri().to_string().starts_with(public_endpoint) {
				return Ok(next.run(req).await);
			}
		}

		let authorization = req
			.headers()
			.get(header::AUTHORIZATION)
			.ok_or(ServerError::Unauthorized)?
			.to_str()
			.map_err(|_| ServerError::Unauthorized)?;

		let token = match authorization.split_once(' ') {
			Some(("Bearer", contents)) => contents.to_string(),
			_ => return Err(ServerError::Unauthorized),
		};

		match ApiKey::get_by_hashing(app.db(), &token)
			.await
			.map_err(|_| ServerError::Unauthorized)?
		{
			Some(api_key) if api_key.is_active => {
				if api_key.secret_key.is_some() {
					ApiKey::hide_key(app.db(), api_key.api_key_id).await?;
				}

				Ok(next.run(req).await)
			}
			_ => Err(ServerError::Unauthorized),
		}
	}

	#[tracing::instrument(name = "server", skip_all)]
	pub async fn start(&self) -> Result<()> {
		let settings = self.app.settings.clone();

		async fn handle_404() -> ServerResult<'static, StatusCode> {
			Err(ServerError::NotFound)
		}

		async fn handle_timeout_error(
			method: Method,
			uri: Uri,
			_err: BoxError,
		) -> ServerResult<'static, StatusCode> {
			Err(ServerError::Internal { error: Report::msg(format!("`{method} {uri}` timed out")) })
		}

		let app = Router::new()
			.merge(handlers::get_routes())
			.route_layer(middleware::from_fn_with_state(self.app.clone(), Self::auth))
			.fallback(handle_404)
			.layer(
				ServiceBuilder::new()
					.layer(HandleErrorLayer::new(handle_timeout_error))
					.timeout(Duration::from_secs(30)),
			)
			.layer(
				TraceLayer::new_for_http()
					.make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
					.on_request(())
					.on_response(
						trace::DefaultOnResponse::new()
							.include_headers(true)
							.latency_unit(LatencyUnit::Millis),
					),
			)
			.with_state(self.app.clone());

		if let Some(ip_addr) = settings.ip_addr {
			let mut listener = None;

			let ports_to_try: Vec<u16> = if settings.port == 2277 {
				let mut ports = vec![2277];
				ports.extend(2278..2300);
				ports
			} else {
				vec![settings.port]
			};

			for port in &ports_to_try {
				let ip_addr = SocketAddr::new(ip_addr, *port);

				match TcpListener::bind(&ip_addr).await {
					Err(_) => {
						warn!("tried listening on port {}", *port);

						if *port == *ports_to_try.last().unwrap() {
							quit(AppError::ServerStartup {
								error: "ran out of ports to try".into(),
							});
						}
					}
					Ok(l) => {
						info!("listening on {ip_addr}…");

						listener = Some(l);
						break;
					}
				}
			}

			if let Some(listener) = listener {
				self.app.set_is_ready();
				axum::serve(listener, app).with_graceful_shutdown(Self::shutdown_signal()).await?;
			}
		}

		Ok(())
	}

	async fn shutdown_signal() {
		let ctrl_c = async {
			if signal::ctrl_c().await.is_err() {
				quit(AppError::SignalHandler);
			}
		};

		#[cfg(unix)]
		let terminate = async {
			match signal::unix::signal(SignalKind::terminate()) {
				Ok(mut signal) => {
					signal.recv().await;
				}
				_ => quit(AppError::SignalHandler),
			};
		};

		#[cfg(not(unix))]
		let terminate = future::pending::<()>();

		tokio::select! {
			_ = ctrl_c => {},
			_ = terminate => {},
		}
	}
}
