use eyre::{Result};
use std::{
	time::SystemTime,
};
use std::process;
use tokio::{
	sync::{watch::Receiver},
	time::{sleep, Duration},
};

use crate::{IndexType, Indexer};
// use barreleye_common::{
// 	chain::WarehouseData,
// 	models::{Config, ConfigKey},
// 	BlockHeight,
// };

impl Indexer {
	pub async fn extract(&self, mut _networks_updated: Receiver<SystemTime>) -> Result<()> {
		// let mut warehouse_data = WarehouseData::new();
		// let mut config_key_map = HashMap::<ConfigKey, serde_json::Value>::new();
		let mut started_indexing = false;

		loop {
		// 'indexing: loop {
			if !self.app.is_leading() {
				if started_indexing {
					self.log(IndexType::Extract, false, "Stopping…");
				}

				started_indexing = false;
				sleep(Duration::from_secs(1)).await;
				continue;
			}

			if !started_indexing {
				started_indexing = true;
				self.log(IndexType::Extract, false, "Starting…");
			}

			if self.app.should_reconnect().await? {
				self.app.connect_networks(true).await?;
			}

			if self.app.networks.read().await.is_empty() {
				self.log(IndexType::Extract, false, "No active networks. Standing by…");
				sleep(Duration::from_secs(5)).await;
				continue;
			}

			// test

			// let w2 = warehouse2::Warehouse::new("clickhouse..?");
			// let result = transfer2::Transfer::create_many(
			// 	&w2,
			// 	vec![Transfer::new()]
			// ).await?;
			// println!("result {:?}", result);
			process::exit(1);

			// let mut network_params_map = HashMap::new();

			// for (network_id, chain) in self.app.networks.read().await.iter() {
			// 	let nid = *network_id;

			// 	let mut last_read_block =
			// 		Config::get::<_, BlockHeight>(self.app.db(), ConfigKey::IndexerTailSync(nid))
			// 			.await?
			// 			.map(|h| h.value)
			// 			.unwrap_or(0);

			// 	let block_height = {
			// 		let config_key = ConfigKey::BlockHeight(nid);
			// 		match Config::get::<_, BlockHeight>(self.app.db(), config_key).await? {
			// 			Some(hit) if hit.value > last_read_block => hit.value,
			// 			_ => {
			// 				let block_height = chain.get_block_height().await?;

			// 				Config::set::<_, BlockHeight>(self.app.db(), config_key, block_height)
			// 					.await?;

			// 				block_height
			// 			}
			// 		}
			// 	};

            //     println!("yo {:?} {:?}", last_read_block, block_height);


			// }
		}
	}
}
