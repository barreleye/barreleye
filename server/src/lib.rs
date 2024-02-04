use axum::{
	error_handling::HandleErrorLayer,
	extract::{Request, State},
	http::{header, Method, StatusCode, Uri},
	middleware::{self, Next},
	response::Response,
	BoxError, Router,
};
use console::style;
use eyre::{Report, Result};
use signal::unix::SignalKind;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{trace, trace::TraceLayer, LatencyUnit};
use tracing::Level;
use uuid::Uuid;

use crate::errors::ServerError;
use barreleye_common::{
	models::ApiKey, quit, App, AppError, Progress, ProgressReadyType,
	ProgressStep, Warnings,
};

mod errors;
mod handlers;
mod utils;

pub type ServerResult<T> = Result<T, ServerError>;

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
	) -> ServerResult<Response> {
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

		let api_key =
			Uuid::parse_str(&token).map_err(|_| ServerError::Unauthorized)?;

		match ApiKey::get_by_uuid(app.db(), &api_key)
			.await
			.map_err(|_| ServerError::Unauthorized)?
		{
			Some(api_key) if api_key.is_active => Ok(next.run(req).await),
			_ => Err(ServerError::Unauthorized),
		}
	}

	pub async fn start(
		&self,
		warnings: Warnings,
		progress: Progress,
	) -> Result<()> {
		let settings = self.app.settings.clone();

		async fn handle_404() -> ServerResult<StatusCode> {
			Err(ServerError::NotFound)
		}

		async fn handle_timeout_error(
			method: Method,
			uri: Uri,
			_err: BoxError,
		) -> ServerResult<StatusCode> {
			Err(ServerError::Internal {
				error: Report::msg(format!("`{method} {uri}` timed out")),
			})
		}

		let app = Router::new()
			.nest("/", handlers::get_routes())
			.route_layer(middleware::from_fn_with_state(
				self.app.clone(),
				Self::auth,
			))
			.fallback(handle_404)
			.layer(
				ServiceBuilder::new()
					.layer(HandleErrorLayer::new(handle_timeout_error))
					.timeout(Duration::from_secs(30)),
			)
			.layer(
				TraceLayer::new_for_http()
					.make_span_with(
						trace::DefaultMakeSpan::new().level(Level::INFO),
					)
					.on_request(())
					.on_response(
						trace::DefaultOnResponse::new()
							.include_headers(true)
							.latency_unit(LatencyUnit::Millis),
					),
			)
			.with_state(self.app.clone());

		let show_progress = |addr: &str| {
			progress.show(ProgressStep::Ready(
				if self.app.settings.is_indexer && self.app.settings.is_server {
					ProgressReadyType::All(addr.to_string())
				} else {
					ProgressReadyType::Server(addr.to_string())
				},
				warnings,
			))
		};

		if let Some(ip_addr) = settings.ip_addr {
			let ip_addr = SocketAddr::new(ip_addr, settings.port);
			show_progress(&format!("Listening on {}…", style(ip_addr).bold()));

			match TcpListener::bind(&ip_addr).await {
				Err(e) => quit(AppError::ServerStartup {
					url: ip_addr.to_string(),
					error: e.to_string(),
				}),
				Ok(listener) => {
					self.app.set_is_ready();
					axum::serve(listener, app)
						.with_graceful_shutdown(Self::shutdown_signal())
						.await?
				}
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
