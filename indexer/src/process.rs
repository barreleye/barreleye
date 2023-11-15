use eyre::{ErrReport, Result};
use serde_json::{from_value as json_parse, json, Value as JsonValue};
use std::{
	cmp,
	collections::HashMap,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	time::SystemTime,
};
use tokio::{
	sync::{broadcast, mpsc, mpsc::Sender, watch::Receiver},
	task::JoinSet,
	time::{sleep, Duration},
};
use tracing::{debug, info, trace};

use crate::Indexer;
use barreleye_common::{
	chain::{ModuleId, WarehouseData},
	models::{Config, ConfigKey, PrimaryId},
	BlockHeight,
};

#[derive(Clone, Debug)]
struct NetworkRange {
	pub network_id: PrimaryId,
	pub range: (BlockHeight, Option<BlockHeight>),
	pub modules: Vec<ModuleId>,
}

impl NetworkRange {
	pub fn new(
		network_id: PrimaryId,
		min: BlockHeight,
		max: Option<BlockHeight>,
		modules: &[ModuleId],
	) -> Self {
		Self { network_id, range: (min, max), modules: modules.to_vec() }
	}
}

pub struct Pipe {
	config_key: ConfigKey,
	sender: mpsc::Sender<(ConfigKey, JsonValue, WarehouseData, bool)>,
	receipt: mpsc::Receiver<()>,
	pub abort: broadcast::Receiver<()>,
}

impl Pipe {
	pub fn new(
		config_key: ConfigKey,
		sender: mpsc::Sender<(ConfigKey, JsonValue, WarehouseData, bool)>,
		receipt: mpsc::Receiver<()>,
		abort: broadcast::Receiver<()>,
	) -> Self {
		Self { config_key, sender, receipt, abort }
	}

	pub async fn push(
		&mut self,
		config_value: JsonValue,
		warehouse_data: WarehouseData,
		force_commit: bool,
	) -> Result<()> {
		self.sender.send((self.config_key, config_value, warehouse_data, force_commit)).await?;

		tokio::select! {
			_ = self.receipt.recv() => {}
			_ = self.abort.recv() => {}
		}

		Ok(())
	}
}

impl Indexer {
	pub async fn process(&self, mut networks_updated: Receiver<SystemTime>) -> Result<()> {
		let mut warehouse_data = WarehouseData::new();
		let mut config_key_map = HashMap::<ConfigKey, serde_json::Value>::new();
		let mut blocked_and_notified = false;

		'indexing: loop {
			if !self.app.is_leading() {
				sleep(Duration::from_secs(1)).await;
				continue;
			}

			if self.app.should_reconnect().await? {
				self.app.connect_networks(true).await?;
			}

			let mut network_params_map = HashMap::new();
			for (network_id, chain) in self.app.networks.read().await.iter() {
				let nid = *network_id;

				// skip if "sync" step is not done yet
				let copy_step_started = matches!(Config::get::<_, BlockHeight>(
                    self.app.db(),
                    ConfigKey::IndexerSyncTail(nid),
                ).await?, Some(hit) if hit.value > 0);
				let copy_step_synced = Config::get_many::<_, (BlockHeight, BlockHeight)>(
					self.app.db(),
					vec![ConfigKey::IndexerSyncChunk(nid, 0)],
				)
				.await?
				.is_empty();
				if !copy_step_started || !copy_step_synced {
					continue;
				}

				let mut last_processed_block = Config::get::<_, BlockHeight>(
					self.app.db(),
					ConfigKey::IndexerProcessTail(nid),
				)
				.await?
				.map(|h| h.value)
				.unwrap_or(0);

				// if first time, split up network into chunks for faster initial processing
				if last_processed_block == 0
					&& self.app.cpu_count > 0
					&& Config::get_many::<_, (BlockHeight, BlockHeight)>(
						self.app.db(),
						vec![ConfigKey::IndexerProcessChunk(nid, 0)],
					)
					.await?
					.is_empty()
				{
					// tip should be at wherever sync step is (not network's block height)
					let last_synced_block_height = Config::get::<_, BlockHeight>(
						self.app.db(),
						ConfigKey::IndexerSyncTail(nid),
					)
					.await?
					.map(|h| h.value)
					.unwrap_or(0);

					// get initial chunk ranges
					let block_sync_ranges = self
						.get_block_chunk_ranges(last_synced_block_height)?
						.into_iter()
						.map(|(min, max)| (ConfigKey::IndexerProcessChunk(nid, max), (min, max)))
						.collect::<HashMap<_, _>>();

					// create chunk sync indexes
					Config::set_many::<_, (BlockHeight, BlockHeight)>(
						self.app.db(),
						block_sync_ranges,
					)
					.await?;

					// fast-forward last read block to almost `last_synced_block_height`
					last_processed_block = last_synced_block_height - 1;
					Config::set::<_, BlockHeight>(
						self.app.db(),
						ConfigKey::IndexerProcessTail(nid),
						last_processed_block,
					)
					.await?;

					// no need for individual module syncs, so mark all as done
					Config::set_many::<_, u8>(
						self.app.db(),
						chain
							.get_module_ids()
							.into_iter()
							.map(|module_id| {
								let mid = module_id as u16;
								(ConfigKey::IndexerProcessModuleDone(nid, mid), 1u8)
							})
							.collect::<HashMap<_, _>>(),
					)
					.await?;
				}

				// push tail index to process latest blocks (incl all modules)
				network_params_map.insert(
					ConfigKey::IndexerProcessTail(nid),
					NetworkRange::new(nid, last_processed_block, None, &chain.get_module_ids()),
				);

				// push all fast-sync block ranges
				for (config_key, block_range) in Config::get_many::<_, (BlockHeight, BlockHeight)>(
					self.app.db(),
					vec![ConfigKey::IndexerProcessChunk(nid, 0)],
				)
				.await?
				{
					network_params_map.insert(
						config_key,
						NetworkRange::new(
							nid,
							block_range.value.0,
							Some(block_range.value.1),
							&chain.get_module_ids(),
						),
					);
				}

				// push individual modules that need to sync up
				for module_id in chain.get_module_ids().into_iter() {
					let mid = module_id as u16;

					let ck_synced = ConfigKey::IndexerProcessModuleDone(nid, mid);
					if Config::get::<_, u8>(self.app.db(), ck_synced).await?.is_none() {
						let ck_block_range = ConfigKey::IndexerProcessModule(nid, mid);

						let block_range = match Config::get::<_, (BlockHeight, BlockHeight)>(
							self.app.db(),
							ck_block_range,
						)
						.await?
						{
							Some(hit) => hit.value,
							_ => {
								let block_range = (0, last_processed_block);

								if last_processed_block > 0 {
									Config::set::<_, (BlockHeight, BlockHeight)>(
										self.app.db(),
										ck_block_range,
										block_range,
									)
									.await?;
								}

								block_range
							}
						};

						if block_range.0 < block_range.1 {
							network_params_map.insert(
								ck_block_range,
								NetworkRange::new(
									nid,
									block_range.0,
									Some(block_range.1),
									&[module_id],
								),
							);
						}
					}
				}
			}

			if network_params_map.is_empty() {
				if !blocked_and_notified {
					debug!("Waiting… (no fully synced networks yet)");
					blocked_and_notified = true;
				}
				sleep(Duration::from_secs(10)).await;
				continue;
			} else {
				blocked_and_notified = false;
			}

			let (pipe_sender, mut pipe_receiver) = mpsc::channel(network_params_map.len());
			let (abort_sender, _) = broadcast::channel(network_params_map.len());
			let should_keep_going = Arc::new(AtomicBool::new(true));
			let mut receipts = HashMap::<ConfigKey, Sender<()>>::new();

			let thread_count = network_params_map.len();
			debug!("Launching {thread_count} thread(s)…");

			let mut futures = JoinSet::new();
			for (config_key, network_params) in network_params_map.clone().into_iter() {
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
						let mut warehouse_data = WarehouseData::new();

						let mut block_height = network_params.range.0;
						let block_height_max = network_params.range.1;

						let config_value = |block_height| match config_key {
							ConfigKey::IndexerProcessTail(_) => json!(block_height),
							ConfigKey::IndexerProcessChunk(_, _)
							| ConfigKey::IndexerProcessModule(_, _)
								if block_height_max.is_some() =>
							{
								json!((block_height, block_height_max.unwrap()))
							}
							_ => panic!("no return value for {config_key}"),
						};

						while should_keep_going.load(Ordering::SeqCst) {
							match block_height_max {
								Some(block_height_max) if block_height + 1 > block_height_max => {
									// push no matter what (even if no warehouse data) so that
									// config keys get updated
									pipe.push(
										config_value(block_height),
										warehouse_data.clone(),
										true,
									)
									.await?;

									break;
								}
								None => {
									let last_synced_block_height = Config::get::<_, BlockHeight>(
										&db,
										ConfigKey::IndexerSyncTail(nid),
									)
									.await?
									.map(|v| v.value)
									.unwrap_or(0);

									if block_height + 1 > last_synced_block_height {
										// push only if have some warehouse data; otherwise, it's ok
										// if config keys get updated later
										if !warehouse_data.is_empty() {
											pipe.push(
												config_value(block_height),
												warehouse_data.clone(),
												true,
											)
											.await?;
										}

										// wait a bit
										let timeout =
											cmp::min(chain.get_network().block_time, 5_000);
										sleep(Duration::from_millis(timeout as u64)).await;
										continue;
									}
								}
								_ => {}
							}

							block_height += 1;

							let is_done = tokio::select! {
								_ = pipe.abort.recv() => true,
								new_data = chain.process_block(
									storage.clone(),
									block_height,
									network_params.modules.clone(),
								) => match new_data? {
									Some(new_data) => {
										warehouse_data += new_data;
										false
									},
									None => true,
								},
							};

							if is_done || warehouse_data.len() > 100 {
								pipe.push(
									config_value(block_height),
									warehouse_data.clone(),
									false,
								)
								.await?;
								warehouse_data.clear();
							}

							if is_done {
								break;
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
				async move { s.show_process_progress(10).await }
			});

			// process thread returns + their outputs
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
					Some((config_key, config_value, new_data, force_commit)) = pipe_receiver.recv() => {
						if !self.app.is_leading() {
							abort()?;
							break;
						}

						// update results
						warehouse_data += new_data;
						config_key_map.insert(config_key, config_value);

						// batch save in warehouse
						if warehouse_data.should_commit(force_commit) {
							trace!(warehouse = "pushing", records = warehouse_data.len());

							// push to warehouse
							warehouse_data.commit(self.app.warehouse.clone()).await?;

							// commit config marker updates
							for (config_key, config_value) in config_key_map.iter() {
								let db = self.app.db();
								let key = *config_key;
								let value = config_value.clone();

								match config_key {
									ConfigKey::IndexerProcessTail(_) => {
										let value = json_parse::<BlockHeight>(value)?;
										Config::set::<_, BlockHeight>(db, key, value).await?;
									}
									ConfigKey::IndexerProcessChunk(_, _) => {
										let (block_range_min, block_range_max) =
											json_parse::<(BlockHeight, BlockHeight)>(value)?;

										if block_range_min < block_range_max {
											Config::set::<_, (BlockHeight, BlockHeight)>(
												db,
												key,
												(block_range_min, block_range_max),
											)
											.await?;
										} else {
											Config::delete(db, key).await?;
										}
									}
									ConfigKey::IndexerProcessModule(nid, mid) => {
										let value =
											json_parse::<(BlockHeight, BlockHeight)>(value)?;
										Config::set::<_, (BlockHeight, BlockHeight)>(db, key, value)
											.await?;

										if value.0 >= value.1 {
											Config::set::<_, u8>(
												db,
												ConfigKey::IndexerProcessModuleDone(*nid, *mid),
												1,
											)
											.await?;
										}
									}
									ConfigKey::IndexerProcessModuleDone(_, _) => {
										let value = json_parse::<u8>(value)?;
										Config::set::<_, u8>(db, key, value).await?;
									}
									_ => {}
								}
							}

							// cleanup: if `config_key_map` contains a key indicating a certain
							// module has been fully synced, it's safe to delete config for its
							// range markers
							for (config_key, _) in config_key_map.iter() {
								if let ConfigKey::IndexerProcessModuleDone(nid, mid) = config_key {
									let ck_block_range = ConfigKey::IndexerProcessModule(*nid, *mid);
									Config::delete(self.app.db(), ck_block_range).await?;
								}
							}

							// reset config key markers
							config_key_map.clear();
						}

						// release thread so it can keep going
						if let Some(receipt) = receipts.get(&config_key) {
							receipt.send(()).await.unwrap();
						}
					}
				}
			}
		}
	}

	async fn show_process_progress(&self, secs: u64) -> Result<()> {
		loop {
			sleep(Duration::from_secs(secs)).await;

			for (network_id, chain) in self.app.networks.read().await.clone().into_iter() {
				let nid = network_id;
				let mut scores = vec![];

				let block_height =
					Config::get::<_, BlockHeight>(self.app.db(), ConfigKey::BlockHeight(nid))
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
					for (_, block_range) in Config::get_many::<_, (BlockHeight, BlockHeight)>(
						self.app.db(),
						vec![ConfigKey::IndexerProcessChunk(nid, 0)],
					)
					.await?
					{
						done_blocks -= block_range.value.1 - block_range.value.0;
					}

					scores.push(done_blocks as f64 / block_height as f64);
				}

				let progress = scores.iter().sum::<f64>() / scores.len() as f64;
				Config::set::<_, f64>(
					self.app.db(),
					ConfigKey::IndexerProcessProgress(nid),
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
