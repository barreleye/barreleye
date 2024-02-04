use bitcoin::hashes::{self, sha256d::Hash};
use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::{StorageDb, StorageModelTrait};

#[derive(Debug, Clone)]
pub struct Input {
	pub tx_hash: Hash,
	pub previous_output_tx_hash: Hash,
	pub previous_output_vout: u32,
}

impl Input {
	pub fn get_all(
		storage_db: &StorageDb,
		tx_hash: Option<Hash>,
	) -> Result<Vec<Input>> {
		let mut ret = vec![];

		if let Some(path) =
			storage_db.get_path(&ParquetFile::Inputs.to_string())?
		{
			let mut query = format!("SELECT * FROM read_parquet('{path}')");
			if let Some(hash) = tx_hash {
				query.push_str(&format!(" WHERE tx_hash='{hash}'"));
			}

			let mut statement = storage_db.db.prepare(&query)?;
			let mut rows = statement.query([])?;

			while let Some(row) = rows.next()? {
				let tx_hash: Vec<u8> = row.get(0)?;
				let previous_output_tx_hash: Vec<u8> = row.get(1)?;

				ret.push(Input {
					tx_hash: hashes::Hash::from_slice(&tx_hash)?,
					previous_output_tx_hash: hashes::Hash::from_slice(
						&previous_output_tx_hash,
					)?,
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
                tx_hash BLOB NOT NULL,
                previous_output_tx_hash BLOB NOT NULL,
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
				<Hash as AsRef<[u8]>>::as_ref(&self.tx_hash),
				<Hash as AsRef<[u8]>>::as_ref(&self.previous_output_tx_hash),
				self.previous_output_vout
			],
		)?;

		Ok(())
	}
}
