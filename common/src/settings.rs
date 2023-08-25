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
	Mode, S3Service, Sanctions, Warnings, S3,
};

#[derive(Parser, Debug)]
#[command(
	author = "Barreleye",
	version,
	about,
	long_about = None
)]
pub struct Settings {
	/// Mode can be used to run either the server or the indexer. By default both are run in parallel.
	#[arg(help_heading = "Runtime options", long, num_args = 1.., value_delimiter = ',')]
	mode: Vec<Mode>,
	#[arg(skip)]
	pub is_indexer: bool,
	#[arg(skip)]
	pub is_server: bool,

	/// Sanctions monitoring can automatically watch government sanction lists and manage those addresses internally.
	#[arg(help_heading = "Runtime options", long, num_args = 1.., value_delimiter = ',')]
	sanctions: Vec<Sanctions>,

	/// Where to store extracted blockchain data.
	/// Can be either a folder or S3-compatible storage.
	///
	/// Folder eg: file:///path_to_folder
	/// Amazon S3 eg: http://s3.us-east-1.amazonaws.com/bucket_name/
	/// Google Cloud Storage eg: http://storage.googleapis.com/bucket_name/
	/// MinIO eg: http://localhost/bucket_name/
	#[arg(
		help_heading = "Storage options",
		short,
		long,
		verbatim_doc_comment,
		env = "BARRELEYE_STORAGE",
		default_value_t = format!(
			"file://{}",
			utils::project_dir(Some("storage")).display().to_string(),
		),
        value_hint = ValueHint::DirPath,
		value_name = "URL"
	)]
	storage: String,
	#[arg(skip)]
	pub storage_path: Option<PathBuf>,
	#[arg(skip)]
	pub storage_url: Option<S3>,

	#[arg(
		help_heading = "Storage options",
		long,
		env = "BARRELEYE_S3_ACCESS_KEY_ID",
		value_name = "ACCESS_KEY"
	)]
	pub s3_access_key_id: Option<String>,

	#[arg(
		help_heading = "Storage options",
		long,
		env = "BARRELEYE_S3_SECRET_ACCESS_KEY",
		value_name = "SECRET"
	)]
	pub s3_secret_access_key: Option<String>,

	/// Database to connect to. Supports PostgreSQL, MySQL and SQLite.
	///
	/// Postgres eg: postgres://username:password@localhost:5432/database_name
	/// MySQL eg: mysql://username:password@localhost:3306/database_name
	/// SQLite eg: sqlite://database_path?mode=rwc
	#[arg(
		help_heading = "Database options",
		short,
		long,
		verbatim_doc_comment,
		env = "BARRELEYE_DATABASE",
		default_value_t = format!(
			"sqlite://{}?mode=rwc",
			utils::project_dir(Some("db")).display().to_string(),
		),
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

	/// Warehouse for big data. Supports ClickHouse.
	///
	/// ClickHouse eg: http://username:password@localhost:8123/database_name
	#[arg(
		help_heading = "Warehouse options",
		short,
		long,
		env = "BARRELEYE_WAREHOUSE",
		default_value = "http://localhost:8123/barreleye",
		value_name = "URI"
	)]
	pub warehouse: String,
	#[arg(skip)]
	pub warehouse_driver: WarehouseDriver,

	#[arg(
		help_heading = "Server options",
		long,
		default_value = "127.0.0.1",
		value_name = "IP_V4_ADDRESS"
	)]
	http_ipv4: String,
	#[arg(skip)]
	pub ipv4: Option<IpAddr>,

	/// Provide an empty string to disable IPv6.
	#[arg(help_heading = "Server options", long, default_value = "", value_name = "IP_V6_ADDRESS")]
	http_ipv6: String,
	#[arg(skip)]
	pub ipv6: Option<IpAddr>,

	#[arg(help_heading = "Server options", long, default_value_t = 4000, value_name = "PORT")]
	pub http_port: u16,
}

impl Settings {
	pub fn new() -> Result<(Self, Warnings)> {
		let mut settings = Self::parse();
		let warnings = Warnings::new();

		// set is_indexer and is_server
		for mode in settings.mode.iter() {
			if *mode == Mode::Indexer {
				settings.is_indexer = true;
			} else if *mode == Mode::Http {
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

		// test warehouse url
		if Url::parse(&settings.warehouse).is_err() {
			return Err(
				AppError::Config { config: "warehouse", error: "could not parse URL" }.into()
			);
		}

		// parse ipv4
		let invalid_ipv4 =
			AppError::Config { config: "http_ipv4", error: "Could not parse IP v4." };
		settings.ipv4 = if !settings.http_ipv4.is_empty() {
			Some(IpAddr::V4(settings.http_ipv4.parse().map_err(|_| invalid_ipv4.clone())?))
		} else {
			None
		};

		// both ipv4 and ipv6 cannot be empty
		if settings.http_ipv4.is_empty() && settings.http_ipv6.is_empty() {
			return Err(invalid_ipv4.into());
		}

		// parse ipv6
		settings.ipv6 = if !settings.http_ipv6.is_empty() {
			Some(IpAddr::V6(settings.http_ipv6.parse().map_err(|_| AppError::Config {
				config: "http_ipv6",
				error: "Could not parse IP v6.",
			})?))
		} else {
			None
		};

		// test storage
		let folder_prefix = "file://";
		if settings.storage.starts_with('/')
			|| settings.storage.to_lowercase().starts_with(folder_prefix)
		{
			let storage = if settings.storage.to_lowercase().starts_with(folder_prefix) {
				settings.storage[folder_prefix.to_string().len()..].to_string()
			} else {
				settings.storage.clone()
			};

			let path = Path::new(&storage);
			if fs::create_dir_all(path).is_err()
				|| PathBuf::from(path).into_os_string().into_string().is_err()
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
