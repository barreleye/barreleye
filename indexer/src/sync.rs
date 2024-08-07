use eyre::{ErrReport, Result};
use serde_json::{from_value as json_parse, json, Value as JsonValue};
use std::{
	collections::HashMap,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	time::SystemTime,
};
use tokio::{
	sync::{broadcast, mpsc, watch},
	task::JoinSet,
	time::{sleep, Duration},
};
use tracing::{debug, info};

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
	pub fn new(
		network_id: PrimaryId,
		min: BlockHeight,
		max: Option<BlockHeight>,
	) -> Self {
		Self { network_id, range: (min, max) }
	}
}

pub struct Pipe {
	config_key: ConfigKey,
	sender: mpsc::Sender<(ConfigKey, JsonValue)>,
	receipt: mpsc::Receiver<()>,
	pub abort: broadcast::Receiver<()>,
}

impl Pipe {
	pub fn new(
		config_key: ConfigKey,
		sender: mpsc::Sender<(ConfigKey, JsonValue)>,
		receipt: mpsc::Receiver<()>,
		abort: broadcast::Receiver<()>,
	) -> Self {
		Self { config_key, sender, receipt, abort }
	}

	pub async fn push(&mut self, config_value: JsonValue) -> Result<()> {
		self.sender.send((self.config_key, config_value)).await?;

		tokio::select! {
			_ = self.receipt.recv() => {}
			_ = self.abort.recv() => {}
		}

		Ok(())
	}
}

impl Indexer {
	pub async fn sync(
		&self,
		mut networks_updated: watch::Receiver<SystemTime>,
	) -> Result<()> {
		'indexing: loop {
			if !self.app.is_leading() {
				sleep(Duration::from_secs(1)).await;
				continue;
			}

			if self.app.should_reconnect().await? {
				self.app.connect_networks(true).await?;
			}

			let mut network_range_map = HashMap::new();

			for (network_id, _) in self.app.networks.read().await.iter() {
				let nid = *network_id;

				let mut last_copied_block = Config::get::<_, BlockHeight>(
					self.app.db(),
					ConfigKey::IndexerSyncTail(nid),
				)
				.await?
				.map(|h| h.value)
				.unwrap_or(0);

				let block_height = self
					.get_updated_block_height(nid, Some(last_copied_block))
					.await?;

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
						.map(|(min, max)| {
							(ConfigKey::IndexerSyncChunk(nid, max), (min, max))
						})
						.collect::<HashMap<_, _>>();

					// create chunk sync indexes
					Config::set_many::<_, (BlockHeight, BlockHeight)>(
						self.app.db(),
						block_sync_ranges,
					)
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
				network_range_map.insert(
					ConfigKey::IndexerSyncTail(nid),
					NetworkRange::new(nid, last_copied_block, None),
				);

				// push all fast-sync block ranges
				for (config_key, block_range) in
					Config::get_many::<_, (BlockHeight, BlockHeight)>(
						self.app.db(),
						vec![ConfigKey::IndexerSyncChunk(nid, 0)],
					)
					.await?
				{
					network_range_map.insert(
						config_key,
						NetworkRange::new(
							nid,
							block_range.value.0,
							Some(block_range.value.1),
						),
					);
				}
			}

			if network_range_map.is_empty() {
				sleep(Duration::from_secs(1)).await;
				continue;
			}

			let (pipe_sender, mut pipe_receiver) =
				mpsc::channel(network_range_map.len());
			let (abort_sender, _) = broadcast::channel(network_range_map.len());
			let should_keep_going = Arc::new(AtomicBool::new(true));
			let mut receipts = HashMap::<ConfigKey, mpsc::Sender<()>>::new();

			let thread_count = network_range_map.len();
			debug!("Launching {thread_count} thread(s)…");

			let mut futures = JoinSet::new();
			for (config_key, network_params) in
				network_range_map.clone().into_iter()
			{
				let (rtx, receipt) = mpsc::channel(1);
				receipts.insert(config_key, rtx);

				futures.spawn({
					let nid = network_params.network_id;
					let networks = self.app.networks.read().await;
					let chain = networks[&network_params.network_id].clone();
					let should_keep_going = should_keep_going.clone();
					let mut pipe = Pipe::new(
						config_key,
						pipe_sender.clone(),
						receipt,
						abort_sender.subscribe(),
					);
					let db = self.app.db().clone();
					let storage = self.app.storage.clone();

					async move {
						let mut block_height = network_params.range.0;
						let block_height_max = network_params.range.1;

						let config_value = |block_height| match config_key {
							ConfigKey::IndexerSyncTail(_) => {
								json!(block_height)
							}
							ConfigKey::IndexerSyncChunk(_, _) => {
								json!((block_height, block_height_max.unwrap()))
							}
							_ => panic!("no return value for {config_key}"),
						};

						while should_keep_going.load(Ordering::SeqCst) {
							match block_height_max {
								Some(block_height_max)
									if block_height + 1 > block_height_max =>
								{
									break;
								}
								None => {
									let config_key =
										ConfigKey::BlockHeight(nid);
									let saved_block_height =
										Config::get::<_, BlockHeight>(
											&db, config_key,
										)
										.await?
										.map(|v| v.value)
										.unwrap_or(0);

									if block_height + 1 > saved_block_height {
										let latest_block_height =
											chain.get_block_height().await?;
										if latest_block_height >
											saved_block_height
										{
											Config::set::<_, BlockHeight>(
												&db,
												config_key,
												latest_block_height,
											)
											.await?;
										} else {
											let timeout =
												chain.get_network().block_time;
											sleep(Duration::from_millis(
												timeout as u64,
											))
											.await;
											continue;
										}
									}
								}
								_ => {}
							}

							block_height += 1;

							let is_done = tokio::select! {
								_ = pipe.abort.recv() => true,
								v = chain.extract_block(storage.clone(), block_height) => !v?,
							};

							if is_done {
								break;
							} else {
								pipe.push(config_value(block_height)).await?;
							}
						}

						Ok::<_, ErrReport>(())
					}
				});
			}

			// drop the original non-cloned
			drop(pipe_sender);

			// periodically show progress
			tokio::spawn({
				let s = self.clone();
				async move { s.show_sync_progress(10).await }
			});

			// process thread returns
			let abort = || -> Result<()> {
				should_keep_going.store(false, Ordering::SeqCst);
				abort_sender.send(())?;
				Ok(())
			};
			loop {
				tokio::select! {
					_ = networks_updated.changed() => {
						debug!("Restarting… (networks updated)");
						abort()?;
						break 'indexing Ok(());
					}
					result = futures.join_next() => {
						if let Some(task_result) = result {
							if let Err(e) = task_result? {
								break 'indexing Err(e);
							}
						} else {
							break;
						}
					}
					Some((config_key, config_value)) = pipe_receiver.recv() => {
						if !self.app.is_leading() {
							abort()?;
							break;
						}

						// release thread so it can keep going
						if let Some(receipt) = receipts.get(&config_key) {
							receipt.send(()).await?;
						}

						// save the new config value
						match config_key {
							ConfigKey::IndexerSyncTail(_) => {
								let value = json_parse::<BlockHeight>(config_value)?;
								Config::set::<_, BlockHeight>(self.app.db(), config_key, value).await?;
							}
							ConfigKey::IndexerSyncChunk(_, _) => {
								let (block_range_min, block_range_max) =
									json_parse::<(BlockHeight, BlockHeight)>(config_value)?;

								if block_range_min < block_range_max {
									Config::set::<_, (BlockHeight, BlockHeight)>(
										self.app.db(),
										config_key,
										(block_range_min, block_range_max),
									)
									.await?;
								} else {
									Config::delete(self.app.db(), config_key).await?;
								}
							}
							_ => panic!("received an unexpected config value {config_key}")
						}
					}
				}
			}
		}
	}

	async fn show_sync_progress(&self, secs: u64) -> Result<()> {
		loop {
			sleep(Duration::from_secs(secs)).await;

			for (network_id, chain) in
				self.app.networks.read().await.clone().into_iter()
			{
				let nid = network_id;
				let mut scores = vec![];

				let block_height = Config::get::<_, BlockHeight>(
					self.app.db(),
					ConfigKey::BlockHeight(nid),
				)
				.await?
				.map(|v| v.value)
				.unwrap_or(0);

				if block_height == 0 {
					scores.push(0.0);
				} else {
					let tail_block = Config::get::<_, BlockHeight>(
						self.app.db(),
						ConfigKey::IndexerSyncTail(nid),
					)
					.await?
					.map(|v| v.value)
					.unwrap_or(0);

					let mut done_blocks = tail_block;
					for (_, block_range) in
						Config::get_many::<_, (BlockHeight, BlockHeight)>(
							self.app.db(),
							vec![ConfigKey::IndexerSyncChunk(nid, 0)],
						)
						.await?
					{
						done_blocks -=
							block_range.value.1 - block_range.value.0;
					}

					scores.push(done_blocks as f64 / block_height as f64);
				}

				let progress = scores.iter().sum::<f64>() / scores.len() as f64;
				Config::set::<_, f64>(
					self.app.db(),
					ConfigKey::IndexerSyncProgress(nid),
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
