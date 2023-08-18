use bitcoin::hashes::sha256d::Hash;
use duckdb::{params, Connection};
use eyre::Result;
use std::str::FromStr;

use super::ParquetFile;
use crate::storage::{StorageDb, StorageModelTrait};

#[derive(Debug, Clone)]
pub struct Input {
	pub tx_hash: Hash,
	pub previous_output_tx_hash: Hash,
	pub previous_output_vout: u32,
}

impl Input {
	pub fn get_all(storage_db: &StorageDb, tx_hash: Option<Hash>) -> Result<Vec<Input>> {
		let mut ret = vec![];

		if let Some(path) = storage_db.get_path("inputs")? {
			let mut query = format!("SELECT * FROM read_parquet('{path}')");
			if let Some(hash) = tx_hash {
				query.push_str(&format!(" WHERE tx_hash='{hash}'"));
			}

			let mut statement = storage_db.db.prepare(&query)?;
			let mut rows = statement.query([])?;

			while let Some(row) = rows.next()? {
				let tx_hash: String = row.get(0)?;
				let previous_output_tx_hash: String = row.get(1)?;

				ret.push(Input {
					tx_hash: Hash::from_str(&tx_hash).unwrap(),
					previous_output_tx_hash: Hash::from_str(&previous_output_tx_hash).unwrap(),
					previous_output_vout: row.get(2)?,
				});
			}
		}

		Ok(ret)
	}
}

impl StorageModelTrait for Input {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                tx_hash VARCHAR NOT NULL,
                previous_output_tx_hash VARCHAR NOT NULL,
                previous_output_vout UINT32 NOT NULL
            );"#,
			ParquetFile::Inputs
		))?;

		Ok(())
	}

	fn insert(&self, db: &Connection) -> Result<()> {
		self.create_table(db)?;

		db.execute(
			&format!(
				r#"INSERT INTO {} (
                    tx_hash, previous_output_tx_hash, previous_output_vout
                ) VALUES (
                    ?, ?, ?
                );"#,
				ParquetFile::Inputs
			),
			params![
				self.tx_hash.to_string(),
				self.previous_output_tx_hash.to_string(),
				self.previous_output_vout
			],
		)?;

		Ok(())
	}
}
