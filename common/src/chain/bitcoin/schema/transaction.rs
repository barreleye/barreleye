use bitcoin::hashes::sha256d::Hash;
use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::StorageModelTrait;

#[derive(Debug)]
pub struct Transaction {
	pub hash: Hash,
	pub version: i32,
	pub lock_time: u32,
	pub input_count: u32,
	pub output_count: u32,
	pub is_coin_base: bool,
}

impl StorageModelTrait for Transaction {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                hash VARCHAR NOT NULL,
                version INT32 NOT NULL,
                lock_time UINT32 NOT NULL,
                input_count UINT32 NOT NULL,
                output_count UINT32 NOT NULL,
				is_coin_base BOOLEAN NOT NULL,
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
                    hash, version, lock_time, input_count, output_count, is_coin_base
                ) VALUES (
                    ?, ?, ?, ?, ?, ?
                );"#,
				ParquetFile::Transactions
			),
			params![
				self.hash.to_string(),
				self.version,
				self.lock_time,
				self.input_count,
				self.output_count,
				self.is_coin_base
			],
		)?;

		Ok(())
	}
}
