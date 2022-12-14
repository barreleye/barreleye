use clickhouse::Row;
use eyre::Result;
use primitive_types::U256;
use serde::{Deserialize, Serialize};

use crate::{models::PrimaryId, u256, warehouse::Warehouse, Address, ChainModuleId};

static TABLE: &str = "tx_amounts";

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

pub use Model as TxAmount;

impl Model {
	pub fn new(
		module_id: ChainModuleId,
		network_id: PrimaryId,
		block_height: u64,
		tx_hash: String,
		address: Address,
		asset_address: Option<Address>,
		amount_in: U256,
		amount_out: U256,
		created_at: u32,
	) -> Self {
		Self {
			module_id: module_id as u16,
			network_id: network_id as u64,
			block_height,
			tx_hash: tx_hash.to_lowercase(),
			address: address.to_string().to_lowercase(),
			asset_address: asset_address.unwrap_or_else(Address::blank).to_string().to_lowercase(),
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
}