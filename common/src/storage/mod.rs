use duckdb::Connection;
use eyre::Result;
use std::{fs, sync::Arc};

use crate::{models::PrimaryId, BlockHeight, Settings};

pub trait StorageModelTrait {
	fn create_table(&self, db: &Connection) -> Result<()>;
	fn insert(&self, db: &Connection) -> Result<()>;
}

#[derive(Debug)]
struct Extension {
	installed: bool,
	loaded: bool,
}

pub struct Storage {
	settings: Arc<Settings>,
}

impl Storage {
	pub fn new(settings: Arc<Settings>) -> Result<Self> {
		let s = Self { settings };

		// install extensions (@TODO see if can bundle them at compile time)
		// `https://github.com/wangfenjin/duckdb-rs/issues/91`
		let _ = s.get_db();

		Ok(s)
	}

	pub fn get(&self, network_id: PrimaryId, block_height: BlockHeight) -> Result<StorageDb> {
		Ok(StorageDb::new(self.settings.clone(), self.get_db()?, network_id, block_height))
	}

	fn get_db(&self) -> Result<Connection> {
		let db = Connection::open_in_memory()?;

		self.install_extension(&db, "parquet")?;

		if self.settings.storage_url.is_some() {
			self.install_extension(&db, "httpfs")?;
		}

		Ok(db)
	}

	fn install_extension(&self, db: &Connection, name: &str) -> Result<()> {
		let mut statement = db.prepare(&format!(
			r"SELECT installed, loaded
			FROM duckdb_extensions()
			WHERE extension_name = '{name}';"
		))?;

		let mut rows = statement
			.query_map([], |r| Ok(Extension { installed: r.get(0)?, loaded: r.get(1)? }))?;

		if let Some(Ok(extension)) = rows.next() {
			let mut commands = vec![];

			if !extension.installed {
				commands.push(format!("INSTALL {name};"));
			}
			if !extension.loaded {
				commands.push(format!("LOAD {name};"));
			}

			if !commands.is_empty() {
				db.execute_batch(&commands.join(""))?;
			}
		}

		Ok(())
	}
}

pub struct StorageDb {
	settings: Arc<Settings>,
	db: Connection,
	network_id: PrimaryId,
	block_height: BlockHeight,
}

impl StorageDb {
	pub fn new(
		settings: Arc<Settings>,
		db: Connection,
		network_id: PrimaryId,
		block_height: BlockHeight,
	) -> Self {
		Self { settings, db, network_id, block_height }
	}

	pub fn insert<T>(&self, model: T) -> Result<()>
	where
		T: StorageModelTrait,
	{
		model.insert(&self.db)
	}

	pub fn commit(&self, files: Vec<String>) -> Result<()> {
		let mut commands = vec![];

		for file in files.into_iter() {
			if let Some(storage_path) = &self.settings.storage_path {
				let absolute_path = storage_path
					.join(self.network_id.to_string())
					.join(self.block_height.to_string());

				// duckdb does not automatically create full path if parts dont exist
				fs::create_dir_all(&absolute_path)?;

				commands.push(format!(
					r"COPY {file} TO '{}/{file}.parquet' (FORMAT PARQUET);",
					absolute_path.into_os_string().into_string().unwrap()
				));
			} else if let Some(_storage_url) = &self.settings.storage_url {
				// @TODO implement copying to s3
			}
		}

		if !commands.is_empty() {
			self.db.execute_batch(&commands.join(""))?;
		}

		Ok(())
	}
}
