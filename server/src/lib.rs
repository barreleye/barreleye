use axum::{
	error_handling::HandleErrorLayer,
	http::{Method, StatusCode, Uri},
	BoxError, Router, Server,
};
use console::style;
use eyre::{bail, Report, Result};
use hyper::server::{accept::Accept, conn::AddrIncoming};
use log::info;
use sea_orm::DatabaseConnection;
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

use barreleye_common::{db, progress, progress::Step, Settings};

mod error;
mod handlers;
use error::ServerError;

#[derive(Clone)]
pub struct ServerState {
	pub db: Arc<DatabaseConnection>,
}

impl ServerState {
	pub fn new(db: Arc<DatabaseConnection>) -> Self {
		ServerState { db }
	}
}

pub type ServerResult<T> = Result<T, ServerError>;

#[tokio::main]
pub async fn start() -> Result<()> {
	let settings = Settings::new()?;

	let db = Arc::new(db::new().await?);
	let shared_state = Arc::new(ServerState::new(db));

	let app = wrap_router(
		Router::with_state(shared_state.clone())
			.merge(handlers::get_routes(shared_state.clone())),
	);

	let port = settings.server.port;
	let ip_v4 = SocketAddr::new(settings.server.ip_v4.parse()?, port);

	if settings.server.ip_v6.is_empty() {
		progress::show(Step::Listening(style(ip_v4).bold().to_string())).await;
		Server::bind(&ip_v4)
			.serve(app.into_make_service())
			.with_graceful_shutdown(shutdown_signal())
			.await?;
	} else {
		let ip_v6 = SocketAddr::new(settings.server.ip_v6.parse()?, port);

		let listeners = CombinedIncoming {
			a: AddrIncoming::bind(&ip_v4)
				.or_else(|e| bail!(e.into_cause().unwrap()))?,
			b: AddrIncoming::bind(&ip_v6)
				.or_else(|e| bail!(e.into_cause().unwrap()))?,
		};

		progress::show(Step::Listening(format!(
			"{} & {}",
			style(ip_v4).bold(),
			style(ip_v6).bold()
		)))
		.await;

		Server::builder(listeners)
			.serve(app.into_make_service())
			.with_graceful_shutdown(shutdown_signal())
			.await?;
	}

	Ok(())
}

pub fn wrap_router(
	router: Router<Arc<ServerState>>,
) -> Router<Arc<ServerState>> {
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

	router.fallback(handle_404).layer(
		ServiceBuilder::new()
			.layer(HandleErrorLayer::new(handle_timeout_error))
			.timeout(Duration::from_secs(30)),
	)
}

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
		if let Poll::Ready(Some(value)) = Pin::new(&mut self.a).poll_accept(cx)
		{
			return Poll::Ready(Some(value));
		}

		if let Poll::Ready(Some(value)) = Pin::new(&mut self.b).poll_accept(cx)
		{
			return Poll::Ready(Some(value));
		}

		Poll::Pending
	}
}

async fn shutdown_signal() {
	let ctrl_c = async {
		signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
	};

	#[cfg(unix)]
	let terminate = async {
		signal::unix::signal(SignalKind::terminate())
			.expect("Failed to install signal handler")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
	let terminate = future::pending::<()>();

	tokio::select! {
		_ = ctrl_c => {},
		_ = terminate => {},
	}

	info!("");
	info!("SIGINT received; shutting down 👋");
}
