use clickhouse::Row;
use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::{
	chain::{u256, U256},
	models::PrimaryIds,
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
		// @TODO until I256 is implemented, doing this hacky "group by"
		// statement ideally: "SELECT ?fields FROM {TABLE} WHERE address IN ?"

		addresses.sort_unstable();
		addresses.dedup();

		let formatted_addresses =
			addresses.iter().map(|addr| format!("'{}'", addr)).collect::<Vec<_>>().join(", ");

		warehouse
			.select(&format!(
				r#"
					SELECT *
					FROM (
	                    SELECT
	                        network_id,
	                        address,
	                        asset_address,
	                        SUM(balance) as balance
	                    FROM {TABLE}
	                    WHERE address IN ({formatted_addresses})
	                    GROUP BY (network_id, address, asset_address)
					)
					WHERE balance >= 0
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
