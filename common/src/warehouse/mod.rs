use clickhouse::Client as ClickHouseClient;
use derive_more::Display;
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{utils, Settings};

#[derive(Display, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum Driver {
	#[default]
	#[serde(rename = "clickhouse")]
	#[display("ClickHouse")]
	ClickHouse,
}

pub struct ClickHouse {
	client: ClickHouseClient,
}

pub struct Warehouse {
	url_without_database: String,
	db_name: String,
	clickhouse: ClickHouse,
}

impl Warehouse {
	pub async fn new(settings: Arc<Settings>) -> Result<Self> {
		let (url_without_database, db_name) =
			utils::without_pathname(&settings.warehouse);

		// create db if doesn't exist + check that connection is good
		ClickHouseClient::default()
			.with_url(url_without_database.clone())
			.query(&format!("CREATE DATABASE IF NOT EXISTS {db_name};"))
			.execute()
			.await
			.wrap_err(url_without_database.clone())?;

		Ok(Self {
			url_without_database: url_without_database.clone(),
			db_name: db_name.clone(),
			clickhouse: ClickHouse {
				client: ClickHouseClient::default()
					.with_url(url_without_database)
					.with_database(db_name),
			},
		})
	}

	pub async fn run_migrations(&self) -> Result<()> {
		self.clickhouse
			.client
			.query(&format!(
				r#"
					CREATE TABLE IF NOT EXISTS {}.transfers
					(
						uuid UUID,
						module_id UInt16,
						network_id UInt64,
						block_height UInt64,
						tx_hash String,
						from_address String,
						to_address String,
						asset_address String,
						relative_amount UInt256,
						batch_amount UInt256,
						created_at DateTime
					)
					ENGINE = ReplacingMergeTree
					ORDER BY (
						module_id,
						network_id,
						block_height,
						tx_hash,
						from_address,
						to_address,
						asset_address,
						relative_amount,
						batch_amount
					)
					PARTITION BY toYYYYMM(created_at);
				"#,
				self.db_name
			))
			.execute()
			.await
			.wrap_err(self.url_without_database.clone())?;

		self.clickhouse
			.client
			.query(&format!(
				r#"
					CREATE TABLE IF NOT EXISTS {}.amounts
					(
						module_id UInt16,
						network_id UInt64,
						block_height UInt64,
						tx_hash String,
						address String,
						asset_address String,
						amount_in UInt256,
						amount_out UInt256,
						created_at DateTime
					)
					ENGINE = ReplacingMergeTree
					ORDER BY (
						network_id,
						block_height,
						tx_hash,
						address,
						asset_address
					)
					PARTITION BY toYYYYMM(created_at);
				"#,
				self.db_name
			))
			.execute()
			.await
			.wrap_err(self.url_without_database.clone())?;

		self.clickhouse
			.client
			.query(&format!(
				r#"
					CREATE MATERIALIZED VIEW IF NOT EXISTS {}.balances
					ENGINE = SummingMergeTree
					PARTITION BY network_id
					ORDER BY (network_id, address, asset_address)
					POPULATE AS
					SELECT
						network_id,
					    address,
					    asset_address,
					    (amount_in - amount_out) as balance
					FROM {}.amounts
					GROUP BY (network_id, address, asset_address, amount_in, amount_out)
				"#,
				self.db_name, self.db_name,
			))
			.execute()
			.await
			.wrap_err(self.url_without_database.clone())?;

		self.clickhouse
			.client
			.query(&format!(
				r#"
					CREATE TABLE IF NOT EXISTS {}.links
					(
						network_id UInt64,
						block_height UInt64,
						from_address String,
						to_address String,
						transfer_uuids Array(UUID),
						created_at DateTime
					)
					ENGINE = ReplacingMergeTree
					ORDER BY (
						network_id,
						block_height,
						from_address,
						to_address,
						transfer_uuids
					)
					PARTITION BY toYYYYMM(created_at);
				"#,
				self.db_name
			))
			.execute()
			.await
			.wrap_err(self.url_without_database.clone())?;

		self.clickhouse
			.client
			.query(&format!(
				r#"
					CREATE MATERIALIZED VIEW IF NOT EXISTS {}.experimental_address_stats
					ENGINE = SummingMergeTree
					PARTITION BY network_id
					ORDER BY (address, network_id)
					POPULATE AS
					SELECT
					    a.address,
					    a.network_id,
					    a.in,
					    b.out
					FROM
					(
					    SELECT
					        to_address AS address,
					        network_id,
					        COUNT(from_address) AS in
					    FROM {}.transfers
					    GROUP BY (network_id, to_address)
					) AS a
					LEFT JOIN
					(
					    SELECT
					        from_address AS address,
					        network_id,
					        COUNT(to_address) AS out
					    FROM {}.transfers
					    GROUP BY (network_id, from_address)
					) AS b ON (a.address = b.address) AND (a.network_id = b.network_id)
				"#,
				self.db_name, self.db_name, self.db_name
			))
			.execute()
			.await
			.wrap_err(self.url_without_database.clone())?;

		Ok(())
	}

	pub fn get(&self) -> &ClickHouseClient {
		&self.clickhouse.client
	}
}
