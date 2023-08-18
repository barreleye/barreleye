use bitcoin::hashes::sha256d::Hash;
use bitcoin::{hash_types::TxMerkleNode, BlockHash};
use duckdb::{params, Connection};
use eyre::Result;
use std::str::FromStr;

use super::ParquetFile;
use crate::storage::{StorageDb, StorageModelTrait};

#[derive(Debug, Clone)]
pub struct Block {
	pub hash: BlockHash,
	pub version: i32,
	pub prev_blockhash: BlockHash,
	pub merkle_root: TxMerkleNode,
	pub time: u32,
	pub bits: u32,
	pub nonce: u32,
}

impl Block {
	pub fn get(storage_db: &StorageDb) -> Result<Option<Block>> {
		let mut ret = None;

		if let Some(path) = storage_db.get_path(&ParquetFile::Block.to_string())? {
			let mut statement =
				storage_db.db.prepare(&format!("SELECT * FROM read_parquet('{path}')"))?;
			let mut rows = statement.query([])?;

			if let Some(row) = rows.next()? {
				let hash: String = row.get(0)?;
				let prev_blockhash: String = row.get(2)?;
				let merkle_root: String = row.get(3)?;

				ret = Some(Block {
					hash: BlockHash::from_str(&hash).unwrap(),
					version: row.get(1)?,
					prev_blockhash: BlockHash::from_str(&prev_blockhash).unwrap(),
					merkle_root: TxMerkleNode::from_raw_hash(Hash::from_str(&merkle_root).unwrap()),
					time: row.get(4)?,
					bits: row.get(5)?,
					nonce: row.get(6)?,
				});
			}
		}

		Ok(ret)
	}
}

impl StorageModelTrait for Block {
	fn create_table(&self, db: &Connection) -> Result<()> {
		db.execute_batch(&format!(
			r#"CREATE TEMP TABLE IF NOT EXISTS {} (
                hash VARCHAR NOT NULL,
                version INT32 NOT NULL,
                prev_blockhash VARCHAR NOT NULL,
                merkle_root VARCHAR NOT NULL,
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
				self.hash.to_string(),
				self.version,
				self.prev_blockhash.to_string(),
				self.merkle_root.to_string(),
				self.time,
				self.bits,
				self.nonce,
			],
		)?;

		Ok(())
	}
}
