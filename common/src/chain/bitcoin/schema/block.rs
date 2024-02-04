use bitcoin::{
	blockdata::block::Version, hash_types::TxMerkleNode, hashes::Hash,
	pow::CompactTarget, BlockHash,
};
use duckdb::{params, Connection};
use eyre::Result;

use super::ParquetFile;
use crate::storage::{StorageDb, StorageModelTrait};

#[derive(Debug, Clone)]
pub struct Block {
	pub hash: BlockHash,
	pub version: Version,
	pub prev_blockhash: BlockHash,
	pub merkle_root: TxMerkleNode,
	pub time: u32,
	pub bits: CompactTarget,
	pub nonce: u32,
}

impl Block {
	pub fn get(storage_db: &StorageDb) -> Result<Option<Block>> {
		let mut ret = None;

		if let Some(path) =
			storage_db.get_path(&ParquetFile::Blocks.to_string())?
		{
			let mut statement = storage_db
				.db
				.prepare(&format!("SELECT * FROM read_parquet('{path}')"))?;
			let mut rows = statement.query([])?;

			if let Some(row) = rows.next()? {
				let hash: Vec<u8> = row.get(0)?;
				let prev_blockhash: Vec<u8> = row.get(2)?;
				let merkle_root: Vec<u8> = row.get(3)?;

				ret = Some(Block {
					hash: BlockHash::from_slice(&hash)?,
					version: Version::from_consensus(row.get(1)?),
					prev_blockhash: BlockHash::from_slice(&prev_blockhash)?,
					merkle_root: TxMerkleNode::from_slice(&merkle_root)?,
					time: row.get(4)?,
					bits: CompactTarget::from_consensus(row.get(5)?),
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
                hash BLOB NOT NULL,
                version INT32 NOT NULL,
                prev_blockhash BLOB NOT NULL,
                merkle_root BLOB NOT NULL,
                time UINT32 NOT NULL,
                bits UINT32 NOT NULL,
                nonce UINT32 NOT NULL
            );"#,
			ParquetFile::Blocks
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
				ParquetFile::Blocks
			),
			params![
				<BlockHash as AsRef<[u8]>>::as_ref(&self.hash),
				self.version.to_consensus(),
				<BlockHash as AsRef<[u8]>>::as_ref(&self.prev_blockhash),
				<TxMerkleNode as AsRef<[u8]>>::as_ref(&self.merkle_root),
				self.time,
				self.bits.to_consensus(),
				self.nonce,
			],
		)?;

		Ok(())
	}
}
