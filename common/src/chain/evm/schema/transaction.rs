use duckdb::{params, Connection};
use ethers::{
	abi::AbiEncode,
	types::{H160, H256, U256},
};
use eyre::Result;

use super::ParquetFile;
use crate::storage::StorageModelTrait;

#[derive(Debug)]
pub struct Transaction {
	pub hash: H256,
	pub nonce: U256,
	pub transaction_index: Option<u64>,
	pub from_address: H160,
	pub to_address: Option<H160>,
	pub value: U256,
	pub gas_price: Option<U256>,
	pub gas: U256,
	pub transaction_type: Option<u64>,
	pub chain_id: Option<U256>,
}

impl StorageModelTrait for Transaction {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                hash BLOB NOT NULL,
                nonce VARCHAR NOT NULL,
                transaction_index VARCHAR,
                from_address BLOB NOT NULL,
                to_address BLOB,
                value VARCHAR NOT NULL,
                gas_price VARCHAR,
                gas VARCHAR NOT NULL,
                transaction_type UINT64,
                chain_id VARCHAR
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
                    hash,
                    nonce,
                    transaction_index,
                    from_address,
                    to_address,
                    value,
                    gas_price,
                    gas,
                    transaction_type,
                    chain_id
                ) VALUES (
                    ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                );"#,
				ParquetFile::Transactions
			),
			params![
				self.hash.encode(),
				self.nonce.to_string(),
				self.transaction_index,
				self.from_address.encode(),
				self.to_address.map(|v| v.encode()),
				self.value.to_string(),
				self.gas_price.map(|v| v.to_string()),
				self.gas.to_string(),
				self.transaction_type,
				self.chain_id.map(|v| v.to_string()),
			],
		)?;

		Ok(())
	}
}
