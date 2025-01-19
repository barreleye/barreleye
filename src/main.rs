extern crate dotenvy;

use dotenvy::dotenv;
use eyre::Result;
use std::{borrow::Cow, sync::Arc};
use tokio::{signal, task::JoinSet};
use tracing::{debug, warn};

use barreleye_common::{quit, App, AppError, Db, Settings, Storage, Warehouse};
use barreleye_indexer::Indexer;
use barreleye_server::Server;

mod log;

#[tokio::main]
async fn main() -> Result<()> {
	dotenv().ok();
	log::setup()?;

	let raw_settings = Settings::new().await.unwrap_or_else(|e| {
		quit(match e.downcast_ref::<AppError>() {
			Some(app_error) => app_error.clone(),
			None => AppError::Unexpected { error: Cow::Owned(e.to_string()) },
		})
	});

	let settings = Arc::new(raw_settings);

	let db = Arc::new(Db::new(settings.clone()).await.unwrap_or_else(|url| {
		quit(AppError::Connection {
			service: Cow::Borrowed(&settings.database_driver.to_string()),
			url: Cow::Owned(url.to_string()),
		});
	}));

	let storage = Arc::new(Storage::new(settings.clone()).unwrap_or_else(|url| {
		quit(AppError::Connection {
			service: Cow::Borrowed("storage"),
			url: Cow::Owned(url.to_string()),
		});
	}));

	let warehouse = Arc::new(Warehouse::new(settings.clone()).await.unwrap_or_else(|url| {
		quit(AppError::Connection {
			service: Cow::Borrowed("warehouse"),
			url: Cow::Owned(url.to_string()),
		});
	}));

	debug!("running database migrations");
	db.run_migrations().await?;
	debug!("running warehouse migrations");
	warehouse.run_migrations().await?;

	let app = Arc::new(App::new(settings.clone(), storage, db, warehouse).await?);

	let mut set = JoinSet::new();
	set.spawn(async {
		signal::ctrl_c().await.ok();
		warn!("SIGINT received; shutting down… 👋");
		Ok(())
	});

	if settings.is_indexer {
		debug!("checking blockchain nodes connectivity");
		if let Err(e) = app.connect_networks(false).await {
			quit(AppError::Network { error: Cow::Owned(e.to_string()) });
		}

		set.spawn({
			let a = app.clone();

			async move {
				let indexer = Indexer::new(a);
				indexer.start().await
			}
		});
	}

	if settings.is_server {
		set.spawn({
			let a = app.clone();

			async move {
				let server = Server::new(a);
				server.start().await
			}
		});
	} else {
		app.set_is_ready();
	}

	while let Some(res) = set.join_next().await {
		if let Err(e) = res? {
			quit(match e.downcast_ref::<AppError>() {
				Some(app_error) => app_error.clone(),
				None => AppError::Unexpected { error: Cow::Owned(e.to_string()) },
			});
		}
	}

	Ok(())
}
