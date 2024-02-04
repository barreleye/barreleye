use duckdb::Connection;
use eyre::Result;
use std::{fs, sync::Arc};

use crate::{models::PrimaryId, BlockHeight, Settings};

pub trait StorageModelTrait {
	fn create_table(&self, db: &Connection) -> Result<()>;
	fn insert(&self, db: &Connection) -> Result<()>;
}

pub struct Storage {
	settings: Arc<Settings>,
}

impl Storage {
	pub fn new(settings: Arc<Settings>) -> Result<Self> {
		Ok(Self { settings })
	}

	pub fn get(
		&self,
		network_id: PrimaryId,
		block_height: BlockHeight,
	) -> Result<StorageDb> {
		Ok(StorageDb::new(
			self.settings.clone(),
			self.get_db()?,
			network_id,
			block_height,
		))
	}

	fn get_db(&self) -> Result<Connection> {
		let db = Connection::open_in_memory()?;

		if self.settings.storage_url.is_some() {
			self.set_credentials(&db)?;
		}

		Ok(db)
	}

	fn set_credentials(&self, db: &Connection) -> Result<()> {
		if let Some(s3) = self.settings.storage_url.clone() {
			let mut commands = vec![];

			if let Some(region) = s3.region {
				commands.push(format!("SET s3_region='{region}';"));
			} else if let Some(domain) = s3.domain {
				commands.push(format!("SET s3_endpoint='{domain}';"));
			}

			if let Some(s3_access_key_id) =
				self.settings.s3_access_key_id.clone()
			{
				commands.push(format!(
					"SET s3_access_key_id='{}';",
					s3_access_key_id
				));
			}
			if let Some(s3_secret_access_key) =
				self.settings.s3_secret_access_key.clone()
			{
				commands.push(format!(
					"SET s3_secret_access_key='{}';",
					s3_secret_access_key
				));
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
	pub db: Connection,
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
			if let Some(path) = self.get_path(&file)? {
				commands.push(format!(
					"COPY {file} TO '{}' (FORMAT PARQUET);",
					path,
				));
			}
		}

		if !commands.is_empty() {
			self.db.execute_batch(&commands.join(""))?;
		}

		Ok(())
	}

	pub fn get_path(&self, file: &str) -> Result<Option<String>> {
		let mut ret = None;

		if let Some(storage_path) = &self.settings.storage_path {
			let absolute_path = storage_path
				.join(format!("network_id={}", self.network_id))
				.join(format!("block_height={}", self.block_height));

			// duckdb does not automatically create full path if parts dont
			// exist
			fs::create_dir_all(&absolute_path)?;

			ret = Some(format!(
				"{}/{file}.parquet",
				absolute_path.into_os_string().into_string().unwrap()
			))
		} else if let Some(storage_url) = &self.settings.storage_url {
			let s3_path = format!(
				"{}/network_id={}/block_height={}",
				storage_url.bucket.as_ref().unwrap(),
				self.network_id,
				self.block_height,
			);

			ret = Some(format!("s3://{s3_path}/{file}.parquet"))
		}

		Ok(ret)
	}
}
