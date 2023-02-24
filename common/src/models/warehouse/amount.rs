use clickhouse::Row;
use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::{
	chain::{u256, ModuleId, U256},
	models::{PrimaryId, PrimaryIds},
	warehouse::Warehouse,
};

pub static TABLE: &str = "amounts";

#[derive(PartialEq, Eq, Hash, Debug, Clone, Row, Serialize, Deserialize)]
pub struct Model {
	pub module_id: u16,
	pub network_id: u64,
	pub block_height: u64,
	pub tx_hash: String,
	pub address: String,
	pub asset_address: String,
	#[serde(with = "u256")]
	pub amount_in: U256,
	#[serde(with = "u256")]
	pub amount_out: U256,
	pub created_at: u32,
}

pub use Model as Amount;

impl Model {
	pub fn new(
		module_id: ModuleId,
		network_id: PrimaryId,
		block_height: u64,
		tx_hash: &str,
		address: &str,
		asset_address: Option<String>,
		amount_in: U256,
		amount_out: U256,
		created_at: u32,
	) -> Self {
		Self {
			module_id: module_id as u16,
			network_id: network_id as u64,
			block_height,
			tx_hash: tx_hash.to_string(),
			address: address.to_string(),
			asset_address: asset_address.unwrap_or_default(),
			amount_in,
			amount_out,
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

	pub async fn get_all_network_ids_by_addresses(
		warehouse: &Warehouse,
		mut addresses: Vec<String>,
	) -> Result<PrimaryIds> {
		#[derive(PartialEq, Eq, Hash, Debug, Clone, Row, Serialize, Deserialize)]
		struct Data {
			network_id: u64,
		}

		addresses.sort_unstable();
		addresses.dedup();

		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SELECT DISTINCT network_id
					FROM {TABLE}
					WHERE address IN ?
                "#
			))
			.bind(addresses)
			.fetch_all::<Data>()
			.await?
			.into_iter()
			.map(|d| d.network_id as PrimaryId)
			.collect::<Vec<PrimaryId>>()
			.into())
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
