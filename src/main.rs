use eyre::Result;
use std::sync::Arc;
use tokio::{signal, sync::RwLock, task::JoinSet};

use barreleye_common::{
	quit, App, AppError, Cache, Db, Progress, ProgressStep, Settings, Storage, Warehouse,
};
use barreleye_indexer::Indexer;
use barreleye_server::Server;

// mod log;

#[tokio::main]
async fn main() -> Result<()> {
	env_logger::init();
	// log::setup()?;

	let (raw_settings, mut warnings) = Settings::new().unwrap_or_else(|e| {
		quit(match e.downcast_ref::<AppError>() {
			Some(app_error) => app_error.clone(),
			None => AppError::Unexpected { error: e.to_string() },
		})
	});

	let settings = Arc::new(raw_settings);

	let progress = Progress::new(settings.is_indexer);
	progress.show(ProgressStep::Setup);

	let cache = Arc::new(RwLock::new(Cache::new(settings.clone()).await?));

	let warehouse = Arc::new(Warehouse::new(settings.clone()).await.unwrap_or_else(|url| {
		quit(AppError::WarehouseConnection { url: url.to_string() });
	}));

	let storage = Arc::new(RwLock::new(Storage::new(settings.clone())));

	let db = Arc::new(Db::new(settings.clone()).await.unwrap_or_else(|url| {
		quit(AppError::DatabaseConnection { url: url.to_string() });
	}));

	progress.show(ProgressStep::Migrations);
	warehouse.run_migrations().await?;
	db.run_migrations().await?;

	let app = Arc::new(App::new(settings.clone(), cache, storage, db, warehouse).await?);
	warnings.extend(app.get_warnings().await?);

	let mut set = JoinSet::new();
	set.spawn(async {
		signal::ctrl_c().await.ok();
		println!("\nSIGINT received; bye 👋");
		Ok(())
	});

	if settings.is_indexer {
		progress.show(ProgressStep::Networks);
		if let Err(e) = app.connect_networks(false).await {
			quit(AppError::Network { error: e.to_string() });
		}

		set.spawn({
			let a = app.clone();
			let w = warnings.clone();
			let p = progress.clone();

			async move {
				let indexer = Indexer::new(a);
				indexer.start(w, p).await
			}
		});
	}

	if settings.is_server {
		set.spawn({
			let a = app.clone();
			let w = warnings.clone();
			let p = progress.clone();

			async move {
				let server = Server::new(a);
				server.start(w, p).await
			}
		});
	} else {
		app.set_is_ready();
	}

	while let Some(res) = set.join_next().await {
		if let Err(e) = res? {
			quit(match e.downcast_ref::<AppError>() {
				Some(app_error) => app_error.clone(),
				None => AppError::Unexpected { error: e.to_string() },
			});
		}
	}

	Ok(())
}
