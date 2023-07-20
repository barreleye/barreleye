use axum::{
	error_handling::HandleErrorLayer,
	extract::State,
	http::{header, Method, Request, StatusCode, Uri},
	middleware::{self, Next},
	response::Response,
	BoxError, Router, Server as AxumServer,
};
use console::style;
use eyre::{Report, Result};
use hyper::server::{accept::Accept, conn::AddrIncoming};
use signal::unix::SignalKind;
use std::{
	net::SocketAddr,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
	time::Duration,
};
use tokio::signal;
use tower::ServiceBuilder;
use uuid::Uuid;

use crate::errors::ServerError;
use barreleye_common::{
	models::ApiKey, quit, App, AppError, Progress, ProgressReadyType, ProgressStep, Warnings,
};

mod errors;
mod handlers;
mod utils;

pub type ServerResult<T> = Result<T, ServerError>;

struct CombinedIncoming {
	a: AddrIncoming,
	b: AddrIncoming,
}

impl Accept for CombinedIncoming {
	type Conn = <AddrIncoming as Accept>::Conn;
	type Error = <AddrIncoming as Accept>::Error;

	fn poll_accept(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
		if let Poll::Ready(Some(value)) = Pin::new(&mut self.a).poll_accept(cx) {
			return Poll::Ready(Some(value));
		}

		if let Poll::Ready(Some(value)) = Pin::new(&mut self.b).poll_accept(cx) {
			return Poll::Ready(Some(value));
		}

		Poll::Pending
	}
}

pub struct Server {
	app: Arc<App>,
}

impl Server {
	pub fn new(app: Arc<App>) -> Self {
		Self { app }
	}

	async fn auth<B>(
		State(app): State<Arc<App>>,
		req: Request<B>,
		next: Next<B>,
	) -> ServerResult<Response> {
		for public_endpoint in ["/v0/info", "/v0/upstream"].iter() {
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
			Some((name, contents)) if name == "Bearer" => contents.to_string(),
			_ => return Err(ServerError::Unauthorized),
		};

		let api_key = Uuid::parse_str(&token).map_err(|_| ServerError::Unauthorized)?;

		match ApiKey::get_by_uuid(app.db(), &api_key)
			.await
			.map_err(|_| ServerError::Unauthorized)?
		{
			Some(api_key) if api_key.is_active => Ok(next.run(req).await),
			_ => Err(ServerError::Unauthorized),
		}
	}

	pub async fn start(&self, warnings: Warnings, progress: Progress) -> Result<()> {
		let settings = self.app.settings.clone();

		async fn handle_404() -> ServerResult<StatusCode> {
			Err(ServerError::NotFound)
		}

		async fn handle_timeout_error(
			method: Method,
			uri: Uri,
			_err: BoxError,
		) -> ServerResult<StatusCode> {
			Err(ServerError::Internal { error: Report::msg(format!("`{method} {uri}` timed out")) })
		}

		let app = Router::new()
			.nest("/", handlers::get_routes())
			.route_layer(middleware::from_fn_with_state(self.app.clone(), Self::auth))
			.fallback(handle_404)
			.layer(
				ServiceBuilder::new()
					.layer(HandleErrorLayer::new(handle_timeout_error))
					.timeout(Duration::from_secs(30)),
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

		if let Some(ip_addr) = settings.ipv4.xor(settings.ipv6) {
			let ip_addr = SocketAddr::new(ip_addr, settings.http_port);
			show_progress(&style(ip_addr).bold().to_string());

			match AxumServer::try_bind(&ip_addr) {
				Err(e) => quit(AppError::ServerStartup {
					url: ip_addr.to_string(),
					error: e.message().to_string(),
				}),
				Ok(server) => {
					self.app.set_is_ready();
					server
						.serve(app.into_make_service())
						.with_graceful_shutdown(Self::shutdown_signal())
						.await?
				}
			}
		} else {
			let ipv4 = SocketAddr::new(settings.ipv4.unwrap(), settings.http_port);
			let ipv6 = SocketAddr::new(settings.ipv6.unwrap(), settings.http_port);

			match (AddrIncoming::bind(&ipv4), AddrIncoming::bind(&ipv6)) {
				(Err(e), _) => quit(AppError::ServerStartup {
					url: ipv4.to_string(),
					error: e.message().to_string(),
				}),
				(_, Err(e)) => quit(AppError::ServerStartup {
					url: ipv6.to_string(),
					error: e.message().to_string(),
				}),
				(Ok(a), Ok(b)) => {
					show_progress(&format!("{} & {}", style(ipv4).bold(), style(ipv6).bold()));

					self.app.set_is_ready();
					AxumServer::builder(CombinedIncoming { a, b })
						.serve(app.into_make_service())
						.with_graceful_shutdown(Self::shutdown_signal())
						.await?;
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
