use clickhouse::Row;
use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::{
	chain::{u256, U256},
	models::{PrimaryId, PrimaryIds},
	warehouse::Warehouse,
};

pub static TABLE: &str = "balances";

#[derive(PartialEq, Eq, Hash, Debug, Clone, Row, Serialize, Deserialize)]
pub struct Model {
	pub network_id: u64,
	pub address: String,
	pub asset_address: String,
	#[serde(with = "u256")]
	pub balance: U256,
}

pub use Model as Balance;

impl Model {
	pub async fn get_all_by_addresses(
		warehouse: &Warehouse,
		mut addresses: Vec<String>,
	) -> Result<Vec<Model>> {
		// @TODO until I256 is implemented, doing this hacky "group by" statement
		// ideally: "SELECT ?fields FROM {TABLE} WHERE address IN ?"

		addresses.sort_unstable();
		addresses.dedup();

		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SELECT *
					FROM (
	                    SELECT
	                        network_id,
	                        address,
	                        asset_address,
	                        SUM(balance) as balance
	                    FROM {TABLE}
	                    WHERE address IN ?
	                    GROUP BY (network_id, address, asset_address)
					)
					WHERE balance >= 0
                "#
			))
			.bind(addresses)
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
