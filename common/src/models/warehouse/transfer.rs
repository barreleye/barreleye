use clickhouse::Row;
use eyre::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
	chain::{u256, ModuleId, U256},
	models::{PrimaryId, PrimaryIds},
	utils,
	warehouse::Warehouse,
	BlockHeight,
};

pub static TABLE: &str = "transfers";

#[derive(PartialEq, Eq, Hash, Debug, Clone, Row, Serialize, Deserialize)]
pub struct Model {
	#[serde(with = "clickhouse::serde::uuid")]
	pub uuid: Uuid,
	pub module_id: u16,
	pub network_id: u64,
	pub block_height: u64,
	pub tx_hash: String,
	pub from_address: String,
	pub to_address: String,
	pub asset_address: String,
	#[serde(with = "u256")]
	pub relative_amount: U256,
	#[serde(with = "u256")]
	pub batch_amount: U256,
	pub created_at: u32,
}

pub use Model as Transfer;

impl Model {
	pub fn new(
		module_id: ModuleId,
		network_id: PrimaryId,
		block_height: u64,
		tx_hash: &str,
		from_address: &str,
		to_address: &str,
		asset_address: Option<String>,
		relative_amount: U256,
		batch_amount: U256,
		created_at: u32,
	) -> Self {
		Self {
			uuid: utils::new_uuid(),
			module_id: module_id as u16,
			network_id: network_id as u64,
			block_height,
			tx_hash: tx_hash.to_string(),
			from_address: from_address.to_string(),
			to_address: to_address.to_string(),
			asset_address: asset_address.unwrap_or_default(),
			relative_amount,
			batch_amount,
			created_at,
		}
	}

	pub async fn create_many(warehouse: &Warehouse, models: Vec<Self>) -> Result<()> {
		let mut insert = warehouse.get().insert(TABLE)?;
		for model in models.into_iter() {
			insert.write(&model).await?;
		}

		Ok(insert.end().await?)
	}

	pub async fn get_first_by_source(
		warehouse: &Warehouse,
		network_id: PrimaryId,
		address: &str,
	) -> Result<Option<Self>> {
		let results = warehouse
			.get()
			.query(&format!(
				r#"
					SELECT *
					FROM {TABLE}
					WHERE network_id = ? AND from_address = ?
					ORDER BY created_at ASC
					LIMIT 1
                "#
			))
			.bind(network_id)
			.bind(address.to_string())
			.fetch_all::<Model>()
			.await?;

		Ok(match results.len() {
			0 => None,
			_ => Some(results[0].clone()),
		})
	}

	pub async fn get_all_by_block_range(
		warehouse: &Warehouse,
		network_id: PrimaryId,
		(block_height_min, block_height_max): (BlockHeight, BlockHeight),
	) -> Result<Vec<Self>> {
		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SELECT *
					FROM {TABLE}
					WHERE
						network_id = ? AND
						block_height >= ? AND
						block_height <= ?
					ORDER BY block_height ASC
                "#
			))
			.bind(network_id)
			.bind(block_height_min)
			.bind(block_height_max)
			.fetch_all::<Model>()
			.await?)
	}

	pub async fn get_all_by_uuids(
		warehouse: &Warehouse,
		mut uuids: Vec<Uuid>,
	) -> Result<Vec<Self>> {
		uuids.sort_unstable();
		uuids.dedup();

		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SELECT *
					FROM {TABLE}
					WHERE uuid IN ?
                "#
			))
			.bind(uuids)
			.fetch_all::<Model>()
			.await?)
	}

	pub async fn delete_all_by_network_id(
		warehouse: &Warehouse,
		network_ids: PrimaryIds,
	) -> Result<()> {
		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SET allow_experimental_lightweight_delete = true;
					DELETE FROM {TABLE} WHERE network_id IN ?
                "#
			))
			.bind(network_ids.into_iter().collect::<Vec<PrimaryId>>())
			.execute()
			.await?)
	}
}
