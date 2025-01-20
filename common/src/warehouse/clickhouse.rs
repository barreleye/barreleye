use async_trait::async_trait;
use clickhouse::{Client as ClickHouseClient, Row};
use eyre::{eyre, Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::DriverTrait;
use crate::{utils, Settings};

pub struct ClickHouse {
	url_without_database: String,
	db_name: String,
	client: ClickHouseClient,
}

#[derive(Debug, Row, Deserialize)]
pub struct QueryResult {
	pub network_id: u64,
}

#[derive(Row, Serialize, Deserialize)]
struct DynamicRow {
	#[serde(flatten)]
	data: std::collections::HashMap<String, serde_json::Value>,
}

// @TODO return better error messages like db does (eg: AppError::ConnectionWithCredentials)
#[async_trait]
impl DriverTrait for ClickHouse {
	async fn new(settings: Arc<Settings>) -> Result<Self> {
		let (url_without_database, db_name) = utils::without_pathname(&settings.warehouse);

		ClickHouseClient::default()
			.with_url(url_without_database.clone())
			.query(&format!("CREATE DATABASE IF NOT EXISTS {db_name};"))
			.execute()
			.await
			.wrap_err(url_without_database.clone())?;

		Ok(Self {
			url_without_database: url_without_database.clone(),
			db_name: db_name.clone(),
			client: ClickHouseClient::default()
				.with_url(url_without_database)
				.with_database(db_name),
		})
	}

	async fn run_migrations(&self) -> Result<()> {
		self.client
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

		self.client
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

		self.client
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

		self.client
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

		Ok(())
	}

	async fn insert(&self, table: &str, serialized_data: &[String]) -> Result<()> {
		let mut insert = self.client.insert(table)?;

		for json_str in serialized_data {
			let row: DynamicRow =
				serde_json::from_str(json_str).map_err(|e| eyre!("Failed to parse JSON: {}", e))?;

			insert.write(&row).await.map_err(|e| eyre!("Failed to write row: {}", e))?;
		}

		insert.end().await.map_err(|e| eyre!("Failed to finalize insert: {}", e))?;

		Ok(())
	}

	async fn select(&self, query: &str) -> Result<Vec<String>> {
		let rows: Vec<QueryResult> = self.client.query(query).fetch_all().await?;

		Ok(rows.into_iter().map(|row| row.network_id.to_string()).collect())
	}

	async fn delete(&self, query: &str) -> Result<()> {
		self.client
			.query(query)
			.execute()
			.await
			.map_err(|e| eyre!("Failed to execute delete query: {}", e))?;
		Ok(())
	}
}
