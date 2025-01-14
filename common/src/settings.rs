use clap::{Parser, ValueHint};
use eyre::Result;
use std::{
	fs,
	net::IpAddr,
	path::{Path, PathBuf},
	str::FromStr,
};
use url::Url;

use crate::{
	banner, db::Driver as DatabaseDriver, utils, warehouse::Driver as WarehouseDriver, AppError,
	Mode, S3Service, Warnings, S3,
};

#[derive(Parser, Debug)]
#[command(
	author = "Barreleye",
	version,
	about,
	long_about = Some(r#"Barreleye: multi-chain blockchain indexer and explorer

Barreleye is a powerful tool for indexing and exploring data from multiple blockchains.
It provides a flexible storage system and supports various database options for efficient
data management and analysis."#),
	after_help = "For more information and examples, visit: https://barreleye.org"
)]
pub struct Settings {
	/// Specify the operation mode
	#[arg(
		help_heading = "Runtime options",
		long,
		num_args = 1..,
		value_delimiter = ',',
		default_value = "both"
	)]
	mode: Vec<Mode>,
	#[arg(skip)]
	pub is_indexer: bool,
	#[arg(skip)]
	pub is_server: bool,

	/// Specify the storage location for blockchain data:
	/// - Local folder: /path/to/your/storage/folder
	/// - Amazon S3: https://s3.<region>.amazonaws.com/bucket_name/
	/// - Cloudflare R2: https://<account_id>.r2.cloudflarestorage.com/bucket_name/
	///
	/// The following environment variables can be used to configure S3 credentials:
	/// - BARRELEYE_S3_ACCESS_KEY_ID: S3 access key ID for cloud storage
	/// - BARRELEYE_S3_SECRET_ACCESS_KEY: S3 secret access key for cloud storage
	#[arg(
		help_heading = "Storage options",
		short,
		long,
		verbatim_doc_comment,
		env = "BARRELEYE_STORAGE",
		hide_env_values = true,
		default_value = "file://${HOME}/.barreleye/storage",
        value_hint = ValueHint::DirPath,
		value_name = "URL"
	)]
	storage: String,
	#[arg(skip)]
	pub storage_path: Option<PathBuf>,
	#[arg(skip)]
	pub storage_url: Option<S3>,

	#[arg(long, env = "BARRELEYE_S3_ACCESS_KEY_ID", hide = true)]
	pub s3_access_key_id: Option<String>,

	#[arg(long, env = "BARRELEYE_S3_SECRET_ACCESS_KEY", hide = true)]
	pub s3_secret_access_key: Option<String>,

	/// Specify the database connection URL
	/// Supported databases: SQLite, PostgreSQL, MySQL:
	/// - SQLite: sqlite:///path/to/your/data.db?mode=rwc
	/// - PostgreSQL: postgres://localhost:5432/database_name
	/// - MySQL: mysql://localhost:3306/database_name
	///
	/// The following environment variables can be used to configure credentials
	/// - BARRELEYE_DB_USER: PostgreSQL and MySQL user
	/// - BARRELEYE_DB_PASSWORD: PostgreSQL and MySQL password
	#[arg(
		help_heading = "Database options",
		short,
		long,
		verbatim_doc_comment,
		env = "BARRELEYE_DATABASE",
		hide_env_values = true,
		default_value = "sqlite://${HOME}/.barreleye/data.db?mode=rwc",
        value_hint = ValueHint::DirPath,
		value_name = "URL"
	)]
	pub database: String,
	#[arg(skip)]
	pub database_driver: DatabaseDriver,

	#[arg(help_heading = "Database options", long, default_value_t = 5, value_name = "NUMBER")]
	pub database_min_connections: u32,

	#[arg(help_heading = "Database options", long, default_value_t = 100, value_name = "NUMBER")]
	pub database_max_connections: u32,

	#[arg(help_heading = "Database options", long, default_value_t = 8, value_name = "SECONDS")]
	pub database_connect_timeout: u64,

	#[arg(help_heading = "Database options", long, default_value_t = 8, value_name = "SECONDS")]
	pub database_idle_timeout: u64,

	#[arg(help_heading = "Database options", long, default_value_t = 8, value_name = "SECONDS")]
	pub database_max_lifetime: u64,

	/// Specify the warehouse for storing analytical data
	/// Supported warehouses: DuckDB, ClickHouse:
	/// - DuckDB: /path/to/your/database.db
	/// - ClickHouse: http://localhost:8123/database_name
	///
	/// The following environment variables can be used to configure credentials
	/// - BARRELEYE_WAREHOUSE_USER: ClickHouse user
	/// - BARRELEYE_WAREHOUSE_PASSWORD: ClickHouse password
	#[arg(
		help_heading = "Warehouse options",
		short,
		long,
		verbatim_doc_comment,
		env = "BARRELEYE_WAREHOUSE",
		hide_env_values = true,
		default_value = "file://${HOME}/.barreleye/analytics.db",
		value_name = "URI"
	)]
	pub warehouse: String,
	#[arg(skip)]
	pub warehouse_driver: WarehouseDriver,

	#[arg(
		help_heading = "Server options",
		long,
		default_value = "127.0.0.1",
		value_name = "IP_ADDRESS"
	)]
	/// IP address for the HTTP server
	ip: String,
	#[arg(skip)]
	pub ip_addr: Option<IpAddr>,

	/// Port number for the HTTP server
	#[arg(help_heading = "Server options", long, default_value_t = 2277, value_name = "PORT")]
	pub port: u16,
}

impl Settings {
	pub async fn new() -> Result<(Self, Warnings)> {
		let mut settings = Self::parse();
		let warnings = Warnings::new();

		// set is_indexer and is_server
		for mode in settings.mode.iter() {
			if *mode == Mode::Indexer {
				settings.is_indexer = true;
			} else if *mode == Mode::Http {
				settings.is_server = true;
			} else if *mode == Mode::Both {
				settings.is_indexer = true;
				settings.is_server = true;
			}
		}
		if !settings.is_indexer && !settings.is_server {
			settings.is_indexer = true;
			settings.is_server = true;
		}

		// show banner
		banner::show(settings.is_indexer, settings.is_server)?;

		// set driver for db
		let test_scheme = settings.database.split(':').next().unwrap_or_default();
		if let Ok(driver) = DatabaseDriver::from_str(test_scheme) {
			settings.database_driver = driver;
		} else {
			return Err(AppError::Config { config: "database", error: "invalid URL scheme" }.into());
		}

		// test db database name
		match settings.database_driver {
			DatabaseDriver::PostgreSQL | DatabaseDriver::MySQL
				if !utils::has_pathname(&settings.database) =>
			{
				return Err(AppError::Config {
					config: "database",
					error: "missing database name in the URL",
				}
				.into());
			}
			_ => {}
		}

		// test db url
		if Url::parse(&settings.database).is_err() {
			return Err(
				AppError::Config { config: "database", error: "could not parse URL" }.into()
			);
		}

		// test warehouse
		if let Ok(url) = Url::parse(&settings.warehouse) {
			if url.scheme() == "http" || url.scheme() == "https" {
				settings.warehouse_driver = WarehouseDriver::ClickHouse;
			} else {
				return Err(
					AppError::Config { config: "warehouse", error: "could not parse URL" }.into()
				);
			}
		} else {
			settings.warehouse_driver = WarehouseDriver::DuckDB;
		}

		// test warehouse database name
		match settings.warehouse_driver {
			WarehouseDriver::ClickHouse if !utils::has_pathname(&settings.warehouse) => {
				return Err(AppError::Config {
					config: "warehouse",
					error: "missing database name in the URL",
				}
				.into());
			}
			_ => {}
		}

		// parse ip address
		settings.ip_addr =
			Some(IpAddr::V4(settings.ip.parse().map_err(|_| AppError::Config {
				config: "ip",
				error: "could not parse IP v4.",
			})?));

		// test storage
		let folder_prefix = "file://";
		if settings.storage.starts_with('/') ||
			settings.storage.to_lowercase().starts_with(folder_prefix)
		{
			let storage = if settings.storage.to_lowercase().starts_with(folder_prefix) {
				settings.storage[folder_prefix.to_string().len()..].to_string()
			} else {
				settings.storage.clone()
			};

			let path = Path::new(&storage);
			if fs::create_dir_all(path).is_err() ||
				PathBuf::from(path).into_os_string().into_string().is_err()
			{
				return Err(AppError::Config {
					config: "storage",
					error: "invalid storage directory",
				}
				.into());
			} else {
				settings.storage_path = Some(PathBuf::from(path));
			}
		} else if Url::parse(&settings.storage).is_err() {
			return Err(AppError::Config { config: "storage", error: "invalid storage URL" }.into());
		} else {
			let err = AppError::Config { config: "storage", error: "invalid storage URL" };

			// check url
			if Url::parse(&settings.storage).is_err() {
				return Err(err.into());
			}

			// check that service is known
			let storage_url = S3::from_str(&settings.storage)?;
			if storage_url.service == S3Service::Unknown || storage_url.bucket.is_none() {
				return Err(err.into());
			}

			settings.storage_url = Some(storage_url);
		}

		Ok((settings, warnings))
	}
}
