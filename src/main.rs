extern crate dotenvy;

use console::style;
use dotenvy::dotenv;
use eyre::Result;
use std::sync::Arc;
use tokio::{signal, task::JoinSet};

use barreleye_common::{
	quit, utils, App, AppError, Db, Progress, ProgressStep, Settings, Storage, Warehouse,
};
use barreleye_indexer::Indexer;
use barreleye_server::Server;

mod log;

#[tokio::main]
async fn main() -> Result<()> {
	dotenv().ok();
	log::setup()?;

	let (raw_settings, mut warnings) = Settings::new().unwrap_or_else(|e| {
		quit(match e.downcast_ref::<AppError>() {
			Some(app_error) => app_error.clone(),
			None => AppError::Unexpected { error: e.to_string() },
		})
	});

	let settings = Arc::new(raw_settings);

	let progress = Progress::new(settings.is_indexer);
	progress.show(ProgressStep::Setup);

	let warehouse = Arc::new(Warehouse::new(settings.clone()).await.unwrap_or_else(|url| {
		quit(AppError::WarehouseConnection { url: url.to_string() });
	}));

	let storage = Arc::new(Storage::new(settings.clone()).unwrap_or_else(|url| {
		quit(AppError::StorageConnection { url: url.to_string() });
	}));

	let db = Arc::new(Db::new(settings.clone()).await.unwrap_or_else(|url| {
		quit(AppError::DatabaseConnection { url: url.to_string() });
	}));

	// show connection settings
	fn show_setting(driver: &str, url: &str, tag: &str) {
		println!(
			"          {} {} {}{}",
			style("↳").bold().dim(),
			style(format!("{tag}:")).bold().dim(),
			if !driver.is_empty() {
				format!(
					"{}{}{} ",
					style("[").bold().dim(),
					style(driver).bold(),
					style("]").bold().dim()
				)
			} else {
				"".to_string()
			},
			style(url.to_string()).bold().dim(),
		);
	}
	let storage_type;
	let storage_path;
	if let Some(path) = settings.storage_path.clone() {
		storage_type = "".to_string();
		storage_path = format!("{}", path.display());
	} else if let Some(s3) = settings.storage_url.clone() {
		storage_type = s3.service.to_string();
		storage_path = s3.url;
	} else {
		panic!("storage setting must be set");
	}
	show_setting(&storage_type, &storage_path, "Storage");
	show_setting(
		&settings.database_driver.to_string(),
		&utils::with_masked_auth(&settings.database),
		"Database",
	);
	show_setting(
		&settings.warehouse_driver.to_string(),
		&utils::with_masked_auth(&settings.warehouse),
		"Warehouse",
	);

	progress.show(ProgressStep::Migrations);
	warehouse.run_migrations().await?;
	db.run_migrations().await?;

	let app = Arc::new(App::new(settings.clone(), storage, db, warehouse).await?);
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
