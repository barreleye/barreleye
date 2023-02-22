use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::StorageModelTrait;

#[derive(Debug)]
pub struct Transaction {
	pub hash: String,
	pub version: i32,
	pub lock_time: u32,
	pub inputs: u32,
	pub outputs: u32,
}

impl StorageModelTrait for Transaction {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                hash VARCHAR NOT NULL,
                version INT32 NOT NULL,
                lock_time UINT32 NOT NULL,
                inputs UINT32 NOT NULL,
                outputs UINT32 NOT NULL
            );"#,
			ParquetFile::Transactions
		))?;

		Ok(())
	}

	fn insert(&self, db: &Connection) -> Result<()> {
		self.create_table(db)?;

		db.execute(
			&format!(
				r#"INSERT INTO {} (
                    hash, version, lock_time, inputs, outputs
                ) VALUES (
                    ?, ?, ?, ?, ?
                );"#,
				ParquetFile::Transactions
			),
			params![self.hash, self.version, self.lock_time, self.inputs, self.outputs],
		)?;

		Ok(())
	}
}
