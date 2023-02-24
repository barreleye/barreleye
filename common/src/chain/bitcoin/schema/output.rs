use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::StorageModelTrait;

#[derive(Debug)]
pub struct Output {
	pub value: u64,
	pub script_pubkey: String,
}

impl StorageModelTrait for Output {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                value UINT64 NOT NULL,
                script_pubkey VARCHAR NOT NULL
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
                    value, script_pubkey
                ) VALUES (
                    ?, ?
                );"#,
				ParquetFile::Outputs
			),
			params![self.value, self.script_pubkey],
		)?;

		Ok(())
	}
}
