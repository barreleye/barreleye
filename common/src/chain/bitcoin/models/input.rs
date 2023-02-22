use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::StorageModelTrait;

#[derive(Debug)]
pub struct Input {
	pub previous_output_tx_hash: String,
	pub previous_output_vout: u32,
}

impl StorageModelTrait for Input {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r"CREATE TEMP TABLE IF NOT EXISTS {} (
                previous_output_tx_hash VARCHAR NOT NULL,
                previous_output_vout UINT32 NOT NULL
            );",
			ParquetFile::Inputs
		))?;

		Ok(())
	}

	fn insert(&self, db: &Connection) -> Result<()> {
		self.create_table(db)?;

		db.execute(
			&format!(
				r"INSERT INTO {} (
                    previous_output_tx_hash, previous_output_vout
                ) VALUES (
                    ?, ?
                );",
				ParquetFile::Inputs
			),
			params![self.previous_output_tx_hash, self.previous_output_vout],
		)?;

		Ok(())
	}
}
