use duckdb::{params, Connection};
use ethers::{
	abi::AbiEncode,
	types::{Bytes, H160, H256, U256},
};
use eyre::Result;

use super::ParquetFile;
use crate::storage::StorageModelTrait;

#[derive(Debug)]
pub struct Log {
	pub address: H160,
	// @TODO comma separated strings until `https://github.com/wangfenjin/duckdb-rs/issues/81` is implemented
	pub topics: Vec<H256>,
	pub data: Bytes,
	pub transaction_hash: Option<H256>,
	pub transaction_index: Option<u64>,
	pub log_index: Option<U256>,
	pub transaction_log_index: Option<U256>,
	pub log_type: Option<String>,
	pub removed: Option<bool>,
}

impl StorageModelTrait for Log {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                address VARCHAR,
                topics VARCHAR,
                data VARCHAR,
                transaction_hash VARCHAR,
                transaction_index UINT64,
                log_index VARCHAR,
                transaction_log_index VARCHAR,
                log_type VARCHAR,
                removed BOOLEAN
            );"#,
			ParquetFile::Logs
		))?;

		Ok(())
	}

	fn insert(&self, db: &Connection) -> Result<()> {
		self.create_table(db)?;

		db.execute(
			&format!(
				r#"INSERT INTO {} (
                    address,
                    topics,
                    data,
                    transaction_hash,
                    transaction_index,
                    log_index,
                    transaction_log_index,
                    log_type,
                    removed
                ) VALUES (
                    ?, ?, ?, ?, ?, ?, ?, ?, ?
                );"#,
				ParquetFile::Logs
			),
			params![
				self.address.encode(),
				self.topics
					.iter()
					.map(|v| v.encode_hex())
					.collect::<Vec<String>>()
					.join(","),
				self.data.to_vec(),
				self.transaction_hash.map(|v| v.encode_hex()),
				self.transaction_index,
				self.log_index.map(|v| v.to_string()),
				self.transaction_log_index.map(|v| v.to_string()),
				self.log_type,
				self.removed,
			],
		)?;

		Ok(())
	}
}
