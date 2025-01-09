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

	pub async fn get_first_by_source(
		warehouse: &Warehouse,
		network_id: PrimaryId,
		address: &str,
	) -> Result<Option<Self>> {
		let results: Vec<Self> = warehouse
			.select(&format!(
				r#"
					SELECT *
					FROM {TABLE}
					WHERE network_id = {network_id} AND from_address = {address}
					ORDER BY created_at ASC
					LIMIT 1
                "#
			))
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
		warehouse
			.select(&format!(
				r#"
					SELECT *
					FROM {TABLE}
					WHERE
						network_id = {network_id} AND
						block_height >= {block_height_min} AND
						block_height <= {block_height_max}
					ORDER BY block_height ASC
                "#
			))
			.await
	}

	pub async fn delete_all_by_network_id(
		warehouse: &Warehouse,
		network_ids: PrimaryIds,
	) -> Result<()> {
		let network_ids_string =
			network_ids.into_iter().map(|id| id.to_string()).collect::<Vec<String>>().join(",");

		warehouse
			.delete(&format!(
				r#"
					SET allow_experimental_lightweight_delete = true;
					DELETE FROM {TABLE} WHERE network_id IN ({network_ids_string})
                "#
			))
			.await
	}
}
