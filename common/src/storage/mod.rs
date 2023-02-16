use duckdb::Connection;
use eyre::Result;
use std::sync::Arc;

use crate::Settings;

pub struct Storage {
	_settings: Arc<Settings>,
}

impl Storage {
	pub fn new(_settings: Arc<Settings>) -> Self {
		// check connection
		// parse urls, etc
		Self { _settings }
	}

	pub fn get() -> Result<Connection> {
		let db = Connection::open_in_memory()?;

		db.execute_batch("INSTALL parquet; LOAD parquet;")?;

		Ok(db)
	}
}
