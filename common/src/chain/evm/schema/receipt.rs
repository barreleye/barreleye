use duckdb::{params, Connection};
use ethers::{
	abi::AbiEncode,
	types::{H160, H256, U256},
};
use eyre::Result;

use super::ParquetFile;
use crate::storage::StorageModelTrait;

#[derive(Debug)]
pub struct Receipt {
	pub transaction_hash: H256,
	pub transaction_index: u64,
	pub block_hash: Option<H256>,
	pub block_number: Option<u64>,
	pub from_address: H160,
	pub to_address: Option<H160>,
	pub cumulative_gas_used: U256,
	pub gas_used: Option<U256>,
	pub contract_address: Option<H160>,
	pub logs: u32, // count
	pub status: Option<u64>,
	pub root: Option<H256>,
	pub transaction_type: Option<u64>,
	pub effective_gas_price: Option<U256>,
}

impl StorageModelTrait for Receipt {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                transaction_hash BLOB,
                transaction_index UINT64 NOT NULL,
                block_hash BLOB,
                block_number UINT64,
                from_address BLOB NOT NULL,
                to_address BLOB,
                cumulative_gas_used VARCHAR NOT NULL,
                gas_used VARCHAR,
                contract_address BLOB,
                logs UINT64 NOT NULL,
                status UINT64,
                root BLOB,
                transaction_type UINT64,
                effective_gas_price VARCHAR
            );"#,
			ParquetFile::Receipts
		))?;

		Ok(())
	}

	fn insert(&self, db: &Connection) -> Result<()> {
		self.create_table(db)?;

		db.execute(
			&format!(
				r#"INSERT INTO {} (
                    transaction_hash,
                    transaction_index,
                    block_hash,
                    block_number,
                    from_address,
                    to_address,
                    cumulative_gas_used,
                    gas_used,
                    contract_address,
                    logs,
                    status,
                    root,
                    transaction_type,
                    effective_gas_price
                ) VALUES (
                    ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                );"#,
				ParquetFile::Receipts
			),
			params![
				self.transaction_hash.encode(),
				self.transaction_index,
				self.block_hash.map(|v| v.encode()),
				self.block_number,
				self.from_address.encode(),
				self.to_address.map(|v| v.encode()),
				self.cumulative_gas_used.to_string(),
				self.gas_used.map(|v| v.to_string()),
				self.contract_address.map(|v| v.encode()),
				self.logs,
				self.status,
				self.root.map(|v| v.encode()),
				self.transaction_type,
				self.effective_gas_price.map(|v| v.to_string()),
			],
		)?;

		Ok(())
	}
}
