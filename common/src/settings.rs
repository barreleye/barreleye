use clap::{Parser, ValueHint};
use dirs::home_dir;
use eyre::Result;
use regex::Regex;
use std::{
	fs,
	net::IpAddr,
	path::{Path, PathBuf},
	str::FromStr,
};
use url::Url;

use crate::{
	banner, db::Driver as DatabaseDriver, warehouse::Driver as WarehouseDriver, AppError, Mode,
	S3Service, S3,
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
	#[arg(help_heading = "Runtime Options", long, default_value = "both")]
	mode: Mode,
	#[arg(skip)]
	pub is_indexer: bool,
	#[arg(skip)]
	pub is_server: bool,

	/// Specify the database connection URI
	/// Supported databases: SQLite, PostgreSQL, MySQL:
	/// - SQLite: sqlite:///path/to/your/database.db
	/// - PostgreSQL: postgres://localhost:5432/database_name
	/// - MySQL: mysql://localhost:3306/database_name
	///
	/// The following environment variables can be used to configure credentials
	/// - BARRELEYE_DB_USER: PostgreSQL and MySQL user
	/// - BARRELEYE_DB_PASSWORD: PostgreSQL and MySQL password
	#[arg(
		help_heading = "Database Options",
		short,
		long,
		verbatim_doc_comment,
		env = "BARRELEYE_DATABASE",
		hide_env_values = true,
		default_value = "sqlite://${HOME}/.barreleye/barreleye.sqlite.db",
        value_hint = ValueHint::DirPath,
		value_name = "URI"
	)]
	pub database: String,
	#[arg(skip)]
	pub database_driver: DatabaseDriver,
	#[arg(skip)]
	pub database_uri: Option<Url>,

	#[arg(long, env = "BARRELEYE_DB_USER", hide = true)]
	pub db_user: Option<String>,
	#[arg(long, env = "BARRELEYE_DB_PASSWORD", hide = true)]
	pub db_password: Option<String>,

	#[arg(help_heading = "Database Options", long, default_value_t = 5, value_name = "NUMBER")]
	pub database_min_connections: u32,

	#[arg(help_heading = "Database Options", long, default_value_t = 100, value_name = "NUMBER")]
	pub database_max_connections: u32,

	#[arg(help_heading = "Database Options", long, default_value_t = 8, value_name = "SECONDS")]
	pub database_connect_timeout: u64,

	#[arg(help_heading = "Database Options", long, default_value_t = 8, value_name = "SECONDS")]
	pub database_idle_timeout: u64,

	#[arg(help_heading = "Database Options", long, default_value_t = 8, value_name = "SECONDS")]
	pub database_max_lifetime: u64,

	/// Specify the storage location for blockchain data:
	/// - Local folder: /path/to/your/storage/folder
	/// - Amazon S3: https://s3.<region>.amazonaws.com/bucket_name/
	/// - Cloudflare R2: https://<account_id>.r2.cloudflarestorage.com/bucket_name/
	///
	/// The following environment variables can be used to configure S3 credentials:
	/// - BARRELEYE_S3_ACCESS_KEY_ID: S3 access key ID for cloud storage
	/// - BARRELEYE_S3_SECRET_ACCESS_KEY: S3 secret access key for cloud storage
	#[arg(
		help_heading = "Storage Options",
		short,
		long,
		verbatim_doc_comment,
		env = "BARRELEYE_STORAGE",
		hide_env_values = true,
		default_value = "file://${HOME}/.barreleye/storage",
        value_hint = ValueHint::DirPath,
		value_name = "LOCATION"
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

	/// Specify the warehouse for storing analytical data
	/// Supported warehouses: DuckDB, ClickHouse:
	/// - DuckDB: /path/to/your/database.db
	/// - ClickHouse: http://localhost:8123/database_name
	///
	/// The following environment variables can be used to configure credentials
	/// - BARRELEYE_WAREHOUSE_USER: ClickHouse user
	/// - BARRELEYE_WAREHOUSE_PASSWORD: ClickHouse password
	#[arg(
		help_heading = "Warehouse Options",
		short,
		long,
		verbatim_doc_comment,
		env = "BARRELEYE_WAREHOUSE",
		hide_env_values = true,
		default_value = "${HOME}/.barreleye/barreleye.duckdb.db",
		value_name = "URI"
	)]
	pub warehouse: String,
	#[arg(skip)]
	pub warehouse_driver: WarehouseDriver,
	#[arg(skip)]
	pub warehouse_path: Option<PathBuf>,
	#[arg(skip)]
	pub warehouse_url: Option<Url>,

	#[arg(long, env = "BARRELEYE_WAREHOUSE_USER", hide = true)]
	pub warehouse_user: Option<String>,
	#[arg(long, env = "BARRELEYE_WAREHOUSE_PASSWORD", hide = true)]
	pub warehouse_password: Option<String>,

	#[arg(
		help_heading = "Server Options",
		long,
		default_value = "127.0.0.1",
		value_name = "IPv4_ADDRESS"
	)]
	/// HTTP server bind address
	ip: String,
	#[arg(skip)]
	pub ip_addr: Option<IpAddr>,

	/// Port number for the HTTP server
	#[arg(help_heading = "Server Options", long, default_value_t = 2277, value_name = "PORT")]
	pub port: u16,
}

impl Settings {
	pub async fn new() -> Result<Self> {
		let mut settings = Self::parse();

		// show banner
		banner::show()?;

		// set mode
		match settings.mode {
			Mode::Indexer => {
				settings.is_indexer = true;
			}
			Mode::Http => {
				settings.is_server = true;
			}
			_ => {
				settings.is_indexer = true;
				settings.is_server = true;
			}
		}

		// clean the database path
		let database_path = Self::clean_path("database", settings.database.trim())?;
		let database_path_str = database_path
			.to_str()
			.ok_or(AppError::Config { config: "database".into(), error: "invalid path".into() })?;

		// parse the database URI
		let mut database_parsed_uri = Url::parse(database_path_str).map_err(|_| {
			AppError::Config { config: "database".into(), error: "invalid URI".into() }
		})?;

		// add database credentials
		if let Some(username) = settings.db_user.clone() {
			if database_parsed_uri.set_username(&username).is_err() {
				return Err(AppError::Config {
					config: "database".into(),
					error: "could not set env username".into(),
				}
				.into());
			}
		}
		if let Some(password) = settings.db_password.clone() {
			if database_parsed_uri.set_password(Some(&password)).is_err() {
				return Err(AppError::Config {
					config: "database".into(),
					error: "could not set env password".into(),
				}
				.into());
			}
		}

		// set the database driver
		settings.database_driver =
			database_parsed_uri.scheme().to_ascii_lowercase().parse::<DatabaseDriver>().map_err(
				|_| AppError::Config { config: "database".into(), error: "invalid URI".into() },
			)?;

		match settings.database_driver {
			DatabaseDriver::SQLite => {
				// ensure the directories exist, creating them if necessary
				if let Some(parent) = database_path.parent() {
					fs::create_dir_all(parent).map_err(|_| AppError::Config {
						config: "database".into(),
						error: "invalid path or could not create".into(),
					})?;
				}

				// overwrite query params
				database_parsed_uri.set_query(Some("mode=rwc"));

				// store the processed URI
				settings.database_uri = Some(database_parsed_uri);
			}
			DatabaseDriver::PostgreSQL | DatabaseDriver::MySQL => {
				// check if "database_name" is set in the path
				let database_name = database_parsed_uri
					.path_segments()
					.and_then(|mut segments| segments.next_back())
					.filter(|name| !name.is_empty());

				if database_name.is_none() {
					return Err(AppError::Config {
						config: "database".into(),
						error: "missing database name in the URI".into(),
					}
					.into());
				}

				// store the valid URI
				settings.database_uri = Some(database_parsed_uri);
			}
		}

		// test if storage is a folder
		if Path::new(&settings.storage).is_absolute() ||
			Path::new(&settings.storage).components().next().is_some()
		{
			// clean path
			let path = Self::clean_path("storage", &settings.storage.clone())?;

			// check if the folder exists, create if not
			if !path.exists() && fs::create_dir_all(&path).is_err() {
				return Err(AppError::Config {
					config: "storage".into(),
					error: "invalid path or could not create".into(),
				}
				.into());
			}

			// store the folder path
			settings.storage_path = Some(path.to_path_buf());
		} else if let Ok(parsed_url) = Url::parse(&settings.storage) {
			// ensure the URL has a bucket name
			let has_bucket_name =
				parsed_url.path_segments().and_then(|mut segments| segments.next_back()).is_some();

			if !has_bucket_name {
				return Err(AppError::Config {
					config: "storage".into(),
					error: "missing bucket name in the URI".into(),
				}
				.into());
			} else {
				// check that service is known
				let storage_url = S3::from_str(&settings.storage)?;
				if storage_url.service == S3Service::Unknown || storage_url.bucket.is_none() {
					return Err(AppError::Config {
						config: "storage".into(),
						error: "invalid URL".into(),
					}
					.into());
				}

				settings.storage_url = Some(storage_url);
			}
		} else {
			return Err(AppError::Config {
				config: "storage".into(),
				error: "invalid URL or folder path".into(),
			}
			.into());
		}

		// set warehouse driver
		settings.warehouse_driver = WarehouseDriver::DuckDB;
		if let Ok(url) = Url::parse(&settings.warehouse) {
			if url.scheme() == "http" || url.scheme() == "https" {
				settings.warehouse_driver = WarehouseDriver::ClickHouse;
			} else {
				return Err(AppError::Config {
					config: "warehouse".into(),
					error: "invalid URI".into(),
				}
				.into());
			}
		}

		// test warehouse
		match settings.warehouse_driver {
			WarehouseDriver::DuckDB => {
				let clean_path = Self::clean_path("warehouse", settings.warehouse.trim())?;
				let path = Path::new(&clean_path);

				// check if file exists, create if it doesn't
				if !path.exists() {
					if let Some(parent) = path.parent() {
						fs::create_dir_all(parent).map_err(|_| AppError::Config {
							config: "warehouse".into(),
							error: "invalid path or could not create".into(),
						})?;
					}
				}

				// store the file path
				settings.warehouse_path = Some(PathBuf::from(path));
			}
			WarehouseDriver::ClickHouse => {
				let mut warehouse_url = Url::parse(&settings.warehouse).map_err(|_| {
					AppError::Config { config: "warehouse".into(), error: "invalid URI".into() }
				})?;

				// add credentials
				if let Some(username) = settings.warehouse_user.clone() {
					if warehouse_url.set_username(&username).is_err() {
						return Err(AppError::Config {
							config: "warehouse".into(),
							error: "could not set env username".into(),
						}
						.into());
					}
				}
				if let Some(password) = settings.warehouse_password.clone() {
					if warehouse_url.set_password(Some(&password)).is_err() {
						return Err(AppError::Config {
							config: "warehouse".into(),
							error: "could not set env password".into(),
						}
						.into());
					}
				}

				// check that "database_name" is included in the URL
				if warehouse_url.path().trim_start_matches('/').is_empty() {
					return Err(AppError::Config {
						config: "warehouse".into(),
						error: "missing database name in the URI".into(),
					}
					.into());
				}

				// store the valid URL
				settings.warehouse_url = Some(warehouse_url);
			}
		}

		// parse ip address
		settings.ip_addr = Some(IpAddr::V4(settings.ip.parse().map_err(|_| AppError::Config {
			config: "ip".into(),
			error: "could not parse IPv4".into(),
		})?));

		Ok(settings)
	}

	fn clean_path(config: &str, path_str: &str) -> Result<PathBuf, AppError<'static>> {
		let home_path = home_dir().ok_or(AppError::Config {
			config: config.to_string().into(),
			error: "could not resolve home directory".into(),
		})?;

		// resolve home path as a string
		let home_str = home_path
			.to_str()
			.ok_or(AppError::Config {
				config: config.to_string().into(),
				error: "invalid home path".into(),
			})?
			.to_string();

		// replace `${HOME}` with the home directory path, case-insensitively
		let home_regex = Regex::new(r"(?i)\$\{home\}").map_err(|_| AppError::Config {
			config: config.to_string().into(),
			error: "failed to compile regex for ${HOME}".into(),
		})?;
		let replaced = home_regex.replace_all(path_str, home_str).to_string();

		// remove "file://" prefix, case-insensitively, if exists
		let file_regex = Regex::new(r"(?i)^file://").map_err(|_| AppError::Config {
			config: config.to_string().into(),
			error: "failed to compile regex for file://".into(),
		})?;
		let database_path = file_regex.replace_all(&replaced, "").to_string();

		Ok(Path::new(&database_path).to_path_buf())
	}
}
