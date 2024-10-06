use derive_more::Display;
use eyre::{Result, WrapErr};
use log::LevelFilter;
use sea_orm::{
	ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DatabaseTransaction, DbBackend,
	Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc, time::Duration};

use crate::{utils, Settings};
use migrations::{Migrator, MigratorTrait};

mod migrations;

#[derive(Display, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum Driver {
	#[default]
	#[serde(rename = "sqlite")]
	#[display("SQLite")]
	SQLite,
	#[serde(rename = "postgres")]
	#[display("PostgreSQL")]
	PostgreSQL,
	#[serde(rename = "mysql")]
	#[display("MySQL")]
	MySQL,
}

impl FromStr for Driver {
	type Err = ();

	fn from_str(d: &str) -> Result<Self, Self::Err> {
		match d {
			"sqlite" => Ok(Self::SQLite),
			"postgres" | "postgresql" => Ok(Self::PostgreSQL),
			"mysql" => Ok(Self::MySQL),
			_ => Err(()),
		}
	}
}

pub struct Db {
	db: DatabaseConnection,
}

impl Db {
	pub async fn new(settings: Arc<Settings>) -> Result<Self> {
		let url = settings.database.clone();

		let with_options = |url: String| -> ConnectOptions {
			let mut opt = ConnectOptions::new(url);

			// @TODO for sqlite, max out at 1 connection otherwise
			// writes are not guaranteed to be executed serially
			let (min_connections, max_connections) = match settings.database_driver {
				Driver::SQLite => (1, 1),
				_ => (settings.database_min_connections, settings.database_max_connections),
			};

			opt.max_connections(max_connections)
				.min_connections(min_connections)
				.connect_timeout(Duration::from_secs(settings.database_connect_timeout))
				.idle_timeout(Duration::from_secs(settings.database_idle_timeout))
				.max_lifetime(Duration::from_secs(settings.database_max_lifetime))
				.sqlx_logging(false)
				.sqlx_logging_level(LevelFilter::Warn);

			opt
		};

		let (url_without_database, db_name) = match settings.database_driver {
			Driver::SQLite => (url.clone(), "".to_string()),
			_ => utils::without_pathname(&url),
		};
		let url_with_database = url;

		let conn = Database::connect(with_options(url_without_database.clone()))
			.await
			.wrap_err(url_without_database.clone())?;

		let db = match conn.get_database_backend() {
			DbBackend::MySql => {
				conn.execute(Statement::from_string(
					DbBackend::MySql,
					format!("CREATE DATABASE IF NOT EXISTS `{db_name}`;"),
				))
				.await
				.wrap_err(url_without_database.clone())?;

				Database::connect(with_options(url_with_database.clone()))
					.await
					.wrap_err(url_with_database.clone())?
			}
			DbBackend::Postgres => {
				let result = conn
					.execute(Statement::from_string(
						DbBackend::Postgres,
						format!(
							"SELECT datname FROM pg_catalog.pg_database WHERE datname='{db_name}';"
						),
					))
					.await
					.wrap_err(url_without_database.clone())?;

				if result.rows_affected() == 0 {
					conn.execute(Statement::from_string(
						DbBackend::Postgres,
						format!(r#"CREATE DATABASE "{db_name}";"#),
					))
					.await
					.wrap_err(url_without_database.clone())?;
				}

				Database::connect(with_options(url_with_database.clone()))
					.await
					.wrap_err(url_with_database.clone())?
			}
			DbBackend::Sqlite => conn,
		};

		Ok(Self { db })
	}

	pub async fn run_migrations(&self) -> Result<()> {
		Migrator::up(&self.db, None).await?;
		Ok(())
	}

	pub fn get(&self) -> &DatabaseConnection {
		&self.db
	}

	pub async fn get_tx(&self) -> Result<DatabaseTransaction> {
		Ok(self.db.begin().await?)
	}
}
