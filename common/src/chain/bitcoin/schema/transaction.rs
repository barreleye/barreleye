use bitcoin::{
	blockdata::{locktime::absolute::LockTime, transaction::Version},
	hashes::{self, sha256d::Hash},
};
use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::{StorageDb, StorageModelTrait};

#[derive(Debug, Clone)]
pub struct Transaction {
	pub hash: Hash,
	pub version: Version,
	pub lock_time: LockTime,
	pub input_count: u32,
	pub output_count: u32,
	pub is_coinbase: bool,
}

impl Transaction {
	pub fn get_all(storage_db: &StorageDb) -> Result<Vec<Transaction>> {
		let mut ret = vec![];

		if let Some(path) =
			storage_db.get_path(&ParquetFile::Transactions.to_string())?
		{
			let mut statement = storage_db
				.db
				.prepare(&format!("SELECT * FROM read_parquet('{path}')"))?;
			let mut rows = statement.query([])?;

			while let Some(row) = rows.next()? {
				let hash: Vec<u8> = row.get(0)?;

				ret.push(Transaction {
					hash: hashes::Hash::from_slice(&hash)?,
					version: Version(row.get(1)?),
					lock_time: LockTime::from_consensus(row.get(2)?),
					input_count: row.get(3)?,
					output_count: row.get(4)?,
					is_coinbase: row.get(5)?,
				});
			}
		}

		Ok(ret)
	}
}

impl StorageModelTrait for Transaction {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                hash BLOB NOT NULL,
                version INT32 NOT NULL,
                lock_time UINT32 NOT NULL,
                input_count UINT32 NOT NULL,
                output_count UINT32 NOT NULL,
				is_coinbase BOOLEAN NOT NULL,
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
                    hash, version, lock_time, input_count, output_count, is_coinbase
                ) VALUES (
                    ?, ?, ?, ?, ?, ?
                );"#,
				ParquetFile::Transactions
			),
			params![
				<Hash as AsRef<[u8]>>::as_ref(&self.hash),
				self.version.0,
				self.lock_time.to_consensus_u32(),
				self.input_count,
				self.output_count,
				self.is_coinbase
			],
		)?;

		Ok(())
	}
}
