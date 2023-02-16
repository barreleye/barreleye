use console::style;
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
	pub async fn extract(&self, mut networks_updated: watch::Receiver<SystemTime>) -> Result<()> {
		let mut started_indexing = false;

		'indexing: loop {
			if !self.app.is_leading() {
				if started_indexing {
					info!("Stopping…");
				}

				started_indexing = false;
				sleep(Duration::from_secs(1)).await;
				continue;
			}

			if !started_indexing {
				started_indexing = true;
				info!("Starting…");
			}

			if self.app.should_reconnect().await? {
				self.app.connect_networks(true).await?;
			}

			if self.app.networks.read().await.is_empty() {
				info!("No active networks. Standing by…");
				sleep(Duration::from_secs(5)).await;
				continue;
			}

			let mut network_range_map = HashMap::new();

			for (network_id, chain) in self.app.networks.read().await.iter() {
				let nid = *network_id;

				let mut last_read_block = Config::get::<_, BlockHeight>(
					self.app.db(),
					ConfigKey::IndexerExtractTailSync(nid),
				)
				.await?
				.map(|h| h.value)
				.unwrap_or(0);

				let block_height = {
					let config_key = ConfigKey::BlockHeight(nid);
					match Config::get::<_, BlockHeight>(self.app.db(), config_key).await? {
						Some(hit) if hit.value > last_read_block => hit.value,
						_ => {
							let block_height = chain.get_block_height().await?;

							Config::set::<_, BlockHeight>(self.app.db(), config_key, block_height)
								.await?;

							block_height
						}
					}
				};

				// if first time, split up network into chunks for faster initial syncing
				let chunks = num_cpus::get();
				if last_read_block == 0 &&
					chunks > 0 && Config::get_many::<_, (BlockHeight, BlockHeight)>(
					self.app.db(),
					vec![ConfigKey::IndexerExtractChunkSync(nid, 0)],
				)
				.await?
				.is_empty()
				{
					let chunk_size = ((block_height - 1) as f64 / chunks as f64).floor() as u64;

					// create chunks
					let block_sync_ranges = {
						let mut ret = HashMap::new();

						let mut block_height_min = 0;
						let mut block_height_max = chunk_size;

						for i in 0..chunks {
							if i + 1 == chunks {
								block_height_max = block_height - 1
							}

							ret.insert(
								ConfigKey::IndexerExtractChunkSync(nid, block_height_max),
								(block_height_min, block_height_max),
							);

							block_height_min = block_height_max + 1;
							block_height_max += chunk_size;
						}

						ret
					};

					// create tail-sync indexes
					Config::set_many::<_, (BlockHeight, BlockHeight)>(
						self.app.db(),
						block_sync_ranges,
					)
					.await?;

					// fast-forward last read block to almost block_height
					last_read_block = block_height - 1;
					Config::set::<_, BlockHeight>(
						self.app.db(),
						ConfigKey::IndexerExtractTailSync(nid),
						last_read_block,
					)
					.await?;
				}

				// push tail index to process latest blocks
				network_range_map.insert(
					ConfigKey::IndexerExtractTailSync(nid),
					NetworkRange::new(nid, last_read_block, None),
				);

				// push all fast-sync block ranges
				for (config_key, block_range) in Config::get_many::<_, (BlockHeight, BlockHeight)>(
					self.app.db(),
					vec![ConfigKey::IndexerExtractChunkSync(nid, 0)],
				)
				.await?
				{
					network_range_map.insert(
						config_key,
						NetworkRange::new(nid, block_range.value.0, Some(block_range.value.1)),
					);
				}
			}

			let (pipe_sender, mut pipe_receiver) = mpsc::channel(network_range_map.len());
			let (abort_sender, _) = broadcast::channel(network_range_map.len());
			let should_keep_going = Arc::new(AtomicBool::new(true));
			let mut receipts = HashMap::<ConfigKey, mpsc::Sender<()>>::new();

			let thread_count = network_range_map.len();
			info!("Launching {} thread(s)", style(self.format_number(thread_count)?).bold());

			let mut futures = JoinSet::new();
			for (config_key, network_params) in network_range_map.clone().into_iter() {
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

					async move {
						let mut block_height = network_params.range.0;
						let block_height_max = network_params.range.1;

						let config_value = |block_height| match config_key {
							ConfigKey::IndexerExtractTailSync(_) => json!(block_height),
							ConfigKey::IndexerExtractChunkSync(_, _) => {
								json!((block_height, block_height_max.unwrap()))
							}
							_ => panic!("no return value for {config_key}"),
						};

						while should_keep_going.load(Ordering::SeqCst) {
							match block_height_max {
								Some(block_height_max) if block_height + 1 > block_height_max => {
									break;
								}
								None => {
									let config_key = ConfigKey::BlockHeight(nid);
									let saved_block_height =
										Config::get::<_, BlockHeight>(&db, config_key)
											.await?
											.map(|v| v.value)
											.unwrap_or(0);

									if block_height + 1 > saved_block_height {
										let latest_block_height = chain.get_block_height().await?;
										if latest_block_height > saved_block_height {
											Config::set::<_, BlockHeight>(
												&db,
												config_key,
												latest_block_height,
											)
											.await?;
										} else {
											let timeout = chain.get_network().block_time_ms;
											sleep(Duration::from_millis(timeout as u64)).await;
											continue;
										}
									}
								}
								_ => {}
							}

							block_height += 1;

							let is_done = tokio::select! {
								_ = pipe.abort.recv() => true,
								v = chain.extract_block(block_height) => !v?,
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

			// process thread returns
			let abort = || -> Result<()> {
				should_keep_going.store(false, Ordering::SeqCst);
				abort_sender.send(())?;
				Ok(())
			};
			loop {
				tokio::select! {
					_ = networks_updated.changed() => {
						info!("Restarting… (networks updated)");
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
							receipt.send(()).await.unwrap();
						}

						// save the new config value
						match config_key {
							ConfigKey::IndexerExtractTailSync(_) => {
								let value = json_parse::<BlockHeight>(config_value)?;
								Config::set::<_, BlockHeight>(self.app.db(), config_key, value).await?;
							}
							ConfigKey::IndexerExtractChunkSync(_, _) => {
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
}
