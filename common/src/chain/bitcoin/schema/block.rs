use bitcoin::{hash_types::TxMerkleNode, BlockHash};
use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::StorageModelTrait;

#[derive(Debug)]
pub struct Block {
	pub hash: BlockHash,
	pub version: i32,
	pub prev_blockhash: BlockHash,
	pub merkle_root: TxMerkleNode,
	pub time: u32,
	pub bits: u32,
	pub nonce: u32,
}

impl StorageModelTrait for Block {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                hash BLOB NOT NULL,
                version INT32 NOT NULL,
                prev_blockhash BLOB NOT NULL,
                merkle_root BLOB NOT NULL,
                time UINT32 NOT NULL,
                bits UINT32 NOT NULL,
                nonce UINT32 NOT NULL
            );"#,
			ParquetFile::Block
		))?;

		Ok(())
	}

	fn insert(&self, db: &Connection) -> Result<()> {
		self.create_table(db)?;

		db.execute(
			&format!(
				r#"INSERT INTO {} (
                    hash, version, prev_blockhash, merkle_root, time, bits, nonce
                ) VALUES (
                    ?, ?, ?, ?, ?, ?, ?
                );"#,
				ParquetFile::Block
			),
			params![
				hex::decode(self.hash.to_string())?,
				self.version,
				hex::decode(self.prev_blockhash.to_string())?,
				hex::decode(self.merkle_root.to_string())?,
				self.time,
				self.bits,
				self.nonce,
			],
		)?;

		Ok(())
	}
}
