use eyre::{Report, Result};
use futures::future;
use std::{collections::HashMap, error::Error, time::SystemTime};
use tokio::{
	sync::watch,
	task,
	time::{sleep, Duration},
};
use tracing::info;

use crate::Indexer;
use barreleye_common::{
	models::{Config, ConfigKey, PrimaryId},
	BlockHeight,
};

#[derive(Clone, Debug)]
struct NetworkRange {
	pub network_id: PrimaryId,
	pub range: (BlockHeight, Option<BlockHeight>),
}

impl NetworkRange {
	pub fn new(network_id: PrimaryId, min: BlockHeight, max: Option<BlockHeight>) -> Self {
		Self { network_id, range: (min, max) }
	}
}

impl Indexer {
	pub async fn sync(&self, mut networks_updated: watch::Receiver<SystemTime>) -> Result<()> {
		tokio::spawn({
			let s = self.clone();
			async move { s.show_sync_progress(10).await }
		});

		loop {
			if !self.app.is_leading() {
				sleep(Duration::from_secs(1)).await;
				continue;
			}

			if self.app.should_reconnect().await? {
				self.app.connect_networks(true).await?;
			}

			tokio::select! {
				_ = networks_updated.changed() => {
					continue;
				}

				_ = sleep(Duration::from_secs(0)) => {
					let network_range_map = self.get_network_ranges().await?;

					if network_range_map.is_empty() {
						sleep(Duration::from_secs(3)).await;
						continue;
					}

					let mut tasks = vec![];
					for (_config_key, network_range) in network_range_map.clone().into_iter() {
						let task = task::spawn({
							let networks = self.app.networks.read().await;
							let chain = networks[&network_range.network_id].clone();
							let db = self.app.db().clone();
							let storage = self.app.storage.clone();

							async move {
								match network_range.range {
									(start, Some(end)) => {
										let config_key = ConfigKey::IndexerSyncChunk(network_range.network_id, end);

										for block_height in start..end {
											chain.extract_block(storage.clone(), block_height).await?;

											Config::set::<_, (BlockHeight, BlockHeight)>(
												&db,
												config_key,
												(block_height, end),
											)
											.await?;
										}

										// chunk is done, can delete
										Config::delete(&db, config_key).await?;
									}
									(start, None) => {
										loop {
											let latest_block_height = chain.get_block_height().await?;

											for block_height in start..=latest_block_height {
												chain.extract_block(storage.clone(), block_height).await?;

												let config_key = ConfigKey::IndexerSyncTail(network_range.network_id);
												Config::set::<_, BlockHeight>(&db, config_key, block_height).await?;
											}

											sleep(Duration::from_millis(
												chain.get_network().block_time as u64,
											))
											.await;
										}
									}
								}

								Ok::<(), Box<dyn Error + Send + Sync>>(())
							}

						});

						tasks.push(task);
					}

					let results = future::join_all(tasks).await;
					if let Some(err) = results.into_iter().find(|result| result.is_err()) {
						return Err(Report::msg(format!("A task failed: {:?}", err.as_ref().unwrap_err())));
					}
				}
			}
		}
	}

	async fn get_network_ranges(&self) -> Result<HashMap<ConfigKey, NetworkRange>> {
		let mut ret = HashMap::new();

		for (network_id, _) in self.app.networks.read().await.iter() {
			let nid = *network_id;

			let mut last_copied_block =
				Config::get::<_, BlockHeight>(self.app.db(), ConfigKey::IndexerSyncTail(nid))
					.await?
					.map(|h| h.value)
					.unwrap_or(0);

			let block_height = self.get_updated_block_height(nid, Some(last_copied_block)).await?;

			// if first time, split up network into chunks for faster
			// initial syncing
			if last_copied_block == 0 &&
				self.app.cpu_count > 0 &&
				Config::get_many::<_, (BlockHeight, BlockHeight)>(
					self.app.db(),
					vec![ConfigKey::IndexerSyncChunk(nid, 0)],
				)
				.await?
				.is_empty()
			{
				let block_sync_ranges = self
					.get_block_chunk_ranges(block_height)?
					.into_iter()
					.map(|(min, max)| (ConfigKey::IndexerSyncChunk(nid, max), (min, max)))
					.collect::<HashMap<_, _>>();

				// create chunk sync indexes
				Config::set_many::<_, (BlockHeight, BlockHeight)>(self.app.db(), block_sync_ranges)
					.await?;

				// fast-forward last read block to almost block_height
				last_copied_block = block_height - 1;
				Config::set::<_, BlockHeight>(
					self.app.db(),
					ConfigKey::IndexerSyncTail(nid),
					last_copied_block,
				)
				.await?;
			}

			// push tail index to process latest blocks
			ret.insert(
				ConfigKey::IndexerSyncTail(nid),
				NetworkRange::new(nid, last_copied_block, None),
			);

			// push all fast-sync block ranges
			Config::get_many::<_, (BlockHeight, BlockHeight)>(
				self.app.db(),
				vec![ConfigKey::IndexerSyncChunk(nid, 0)],
			)
			.await?
			.into_iter()
			.for_each(|(config_key, block_range)| {
				ret.insert(
					config_key,
					NetworkRange::new(nid, block_range.value.0, Some(block_range.value.1)),
				);
			});
		}

		Ok(ret)
	}

	async fn show_sync_progress(&self, secs: u64) -> Result<()> {
		loop {
			sleep(Duration::from_secs(secs)).await;

			let networks = self.app.networks.read().await.clone();

			for (network_id, chain) in networks {
				let block_height = Config::get::<_, BlockHeight>(
					self.app.db(),
					ConfigKey::BlockHeight(network_id),
				)
				.await?
				.map(|v| v.value)
				.unwrap_or(0);

				let progress = if block_height == 0 {
					0.0
				} else {
					let tail_block = Config::get::<_, BlockHeight>(
						self.app.db(),
						ConfigKey::IndexerSyncTail(network_id),
					)
					.await?
					.map(|v| v.value)
					.unwrap_or(0);

					let done_blocks = Config::get_many::<_, (BlockHeight, BlockHeight)>(
						self.app.db(),
						vec![ConfigKey::IndexerSyncChunk(network_id, 0)],
					)
					.await?
					.iter()
					.fold(tail_block, |acc, (_, range)| acc - (range.value.1 - range.value.0));

					done_blocks as f64 / block_height as f64
				};

				Config::set::<_, f64>(
					self.app.db(),
					ConfigKey::IndexerSyncProgress(network_id),
					progress,
				)
				.await?;

				info!(
					network = chain.get_network().name,
					progress = (progress * 1000000.0).round() / 1000000.0
				);
			}
		}
	}
}
