use bitcoin::{
	amount::Amount,
	blockdata::script::ScriptBuf,
	hashes::{self, sha256d::Hash},
};
use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::{StorageDb, StorageModelTrait};

#[derive(Debug, Clone)]
pub struct Output {
	pub tx_hash: Hash,
	pub value: Amount,
	pub script_pubkey: ScriptBuf,
}

impl Output {
	pub fn get_all(
		storage_db: &StorageDb,
		tx_hash: Option<Hash>,
	) -> Result<Vec<Output>> {
		let mut ret = vec![];

		if let Some(path) =
			storage_db.get_path(&ParquetFile::Outputs.to_string())?
		{
			let mut query = format!("SELECT * FROM read_parquet('{path}')");
			if let Some(hash) = tx_hash {
				query.push_str(&format!(" WHERE tx_hash='{hash}'"));
			}

			let mut statement = storage_db.db.prepare(&query)?;
			let mut rows = statement.query([])?;

			while let Some(row) = rows.next()? {
				let tx_hash: Vec<u8> = row.get(0)?;
				let script_pubkey: Vec<u8> = row.get(2)?;

				ret.push(Output {
					tx_hash: hashes::Hash::from_slice(&tx_hash)?,
					value: Amount::from_sat(row.get(1)?),
					script_pubkey: ScriptBuf::from_bytes(script_pubkey),
				});
			}
		}

		Ok(ret)
	}
}

impl StorageModelTrait for Output {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                tx_hash BLOB NOT NULL,
                value UINT64 NOT NULL,
                script_pubkey BLOB NOT NULL
            );"#,
			ParquetFile::Outputs
		))?;

		Ok(())
	}

	fn insert(&self, db: &Connection) -> Result<()> {
		self.create_table(db)?;

		db.execute(
			&format!(
				r#"INSERT INTO {} (
                    tx_hash, value, script_pubkey
                ) VALUES (
                    ?, ?, ?
                );"#,
				ParquetFile::Outputs
			),
			params![
				<Hash as AsRef<[u8]>>::as_ref(&self.tx_hash),
				self.value.to_sat(),
				self.script_pubkey.clone().into_bytes(),
			],
		)?;

		Ok(())
	}
}
