use console::style;
use eyre::{ErrReport, Result};
use sea_orm::{ColumnTrait, Condition};
use std::{
	cmp,
	collections::{HashMap, HashSet},
	time::SystemTime,
};
use tokio::{
	sync::watch::Receiver,
	task::JoinSet,
	time::{sleep, Duration},
};

use crate::{IndexType, Indexer};
use barreleye_common::{
	chain::WarehouseData,
	models::{
		Address, AddressColumn, BasicModel, Config, ConfigKey, Link, LinkUuid, Network, PrimaryId,
		PrimaryIds, Transfer,
	},
	BlockHeight,
};

const BLOCKS_PER_LOOP: BlockHeight = 10;
const MAX_ADDRESSES_PER_JOIN_SET: usize = 100;

struct IndexedLinks {
	address: String,
	data: HashMap<String, HashSet<Link>>,
}

impl IndexedLinks {
	pub fn new(address: &str, links: Vec<Link>) -> Self {
		let mut s = Self { address: address.to_string(), data: HashMap::new() };
		s.push(links);
		s
	}

	pub fn get(&self, key: &str) -> Option<&HashSet<Link>> {
		self.data.get(key)
	}

	pub fn contains(&self, key: &str) -> bool {
		self.address == key || self.data.get(key).is_some()
	}

	pub fn push(&mut self, links: Vec<Link>) {
		for link in links.into_iter() {
			if let Some(set) = self.data.get_mut(&link.to_address) {
				set.insert(link);
			} else {
				self.data.insert(link.to_address.clone(), HashSet::from([link]));
			}
		}
	}
}

impl Indexer {
	pub async fn index_upstream(&self, mut networks_updated: Receiver<SystemTime>) -> Result<()> {
		let mut warehouse_data = WarehouseData::new();
		let mut config_key_map = HashMap::<ConfigKey, BlockHeight>::new();
		let mut started_indexing = false;

		'indexing: loop {
			if !self.app.is_leading() {
				if started_indexing {
					self.log(IndexType::Upstream, false, "Stopping…");
				}

				started_indexing = false;
				sleep(Duration::from_secs(1)).await;
				continue;
			}

			if !started_indexing {
				started_indexing = true;
				self.log(IndexType::Upstream, false, "Starting…");
			}

			// get all networks that are not syncing in chunks
			let mut networks = vec![];
			for network in
				Network::get_all_by_env(self.app.db(), self.app.settings.env, Some(false))
					.await?
					.into_iter()
			{
				let tail_is_syncing = Config::exist_by_keywords(
					self.app.db(),
					vec![format!("tail_sync_n{}", network.network_id)],
				);

				let is_actively_syncing = Config::exist_by_keywords(
					self.app.db(),
					vec![
						format!("chunk_sync_n{}", network.network_id),
						format!("module_sync_n{}", network.network_id),
					],
				);

				if tail_is_syncing.await? && !is_actively_syncing.await? {
					networks.push(network);
				}
			}

			// create a map of `network_id` -> `latest_block`
			let block_height_map = {
				let map = networks
					.into_iter()
					.map(|n| (ConfigKey::IndexerTailSync(n.network_id), n.network_id))
					.collect::<HashMap<ConfigKey, PrimaryId>>();

				Config::get_many::<_, BlockHeight>(self.app.db(), map.clone().into_keys().collect())
					.await?
					.into_iter()
					.filter_map(|(config_key, hit)| match hit.value > 0 {
						true => map.get(&config_key).map(|&network_id| (network_id, hit.value)),
						_ => None,
					})
					.collect::<HashMap<PrimaryId, BlockHeight>>()
			};
			if block_height_map.is_empty() {
				self.log(IndexType::Upstream, true, "No fully-synced active networks. Waiting…");
				sleep(Duration::from_secs(5)).await;
				continue;
			}

			// break the link chains that contain newly added addresses in the middle
			let network_ids: PrimaryIds =
				block_height_map.clone().into_keys().collect::<Vec<PrimaryId>>().into();
			self.break_in_new_addresses(network_ids.clone()).await?;

			// fetch all addresses
			let addresses =
				Address::get_all_by_network_ids(self.app.db(), network_ids, Some(false)).await?;
			let all_entity_addresses = addresses
				.iter()
				.map(|a| (a.network_id, a.address.clone()))
				.collect::<HashSet<(PrimaryId, String)>>();
			if addresses.is_empty() {
				self.log(IndexType::Upstream, true, "Nothing to do (no addresses)");
				sleep(Duration::from_secs(5)).await;
				continue;
			}

			// marker to test whether we're all caught up
			let mut is_at_the_tip = true;

			// process a chunk of blocks per address
			let mut futures = JoinSet::new();
			for address in addresses.into_iter() {
				let network_id = address.network_id;
				let latest_block_height = block_height_map[&network_id];

				// get latest block for this address:
				// 1. if in cache -> get it from there
				// 2. if cache is not set -> try reading from configs
				// 3. if not in configs -> fast-forward to 1st interaction from warehouse
				// 4. if not in warehouse -> no need to scan chain; set to chain height
				let config_key = ConfigKey::IndexerUpstreamSync(network_id, address.address_id);
				let block_height = match config_key_map.get(&config_key) {
					Some(&block_height) => block_height,
					_ => {
						let block_height =
							match Config::get::<_, BlockHeight>(self.app.db(), config_key).await? {
								Some(hit) => hit.value,
								_ => Transfer::get_first_by_source(
									&self.app.warehouse,
									network_id,
									&address.address,
								)
								.await?
								.map_or_else(|| latest_block_height, |t| t.block_height - 1),
							};

						config_key_map.insert(config_key, block_height);
						block_height
					}
				};

				// process a new block range if we're not at the tip
				if block_height < latest_block_height {
					let warehouse = self.app.warehouse.clone();
					let min_block_height = block_height + 1;
					let max_block_height =
						cmp::min(block_height + BLOCKS_PER_LOOP, latest_block_height);

					if max_block_height != latest_block_height {
						is_at_the_tip = false;
					}

					let network_entity_addresses =
						all_entity_addresses
							.iter()
							.filter_map(|(nid, address)| {
								if *nid == network_id {
									Some(address.clone())
								} else {
									None
								}
							})
							.collect::<HashSet<String>>();

					futures.spawn({
						let uncommitted_links = warehouse_data
							.clone()
							.links
							.into_iter()
							.filter(|l| l.network_id == network_id as u64)
							.collect::<Vec<Link>>();

						async move {
							let mut ret = WarehouseData::new();

							// seed data from processed but uncommitted links
							let mut indexed_links =
								IndexedLinks::new(&address.address.clone(), uncommitted_links);

							// seed data from warehouse
							indexed_links.push(
								Link::get_all_to_seed_blocks(
									&warehouse,
									network_id,
									(min_block_height, max_block_height),
								)
								.await?,
							);

							// process transfers for a range of blocks
							for transfer in Transfer::get_all_by_block_range(
								&warehouse,
								network_id,
								(min_block_height, max_block_height),
							)
							.await?
							.into_iter()
							{
								if indexed_links.contains(&transfer.from_address) {
									let mut new_links = vec![];

									// create new links
									if let Some(set) = indexed_links.get(&transfer.from_address) {
										// make sure we don't track past existing entity addresses
										if !network_entity_addresses
											.contains(&transfer.from_address)
										{
											// extend branch
											for prev_link in set.iter() {
												let mut transfer_uuids =
													prev_link.transfer_uuids.clone();
												transfer_uuids.push(LinkUuid(transfer.uuid));

												// avoid loopbacks
												if prev_link.from_address != transfer.to_address {
													let link = Link::new(
														address.network_id,
														transfer.block_height,
														&prev_link.from_address,
														&transfer.to_address,
														transfer_uuids,
														transfer.created_at,
													);

													ret.links.insert(link.clone());
													new_links.push(link);
												}
											}
										}
									} else if transfer.from_address != transfer.to_address {
										// start a new branch
										let link = Link::new(
											address.network_id,
											transfer.block_height,
											&transfer.from_address,
											&transfer.to_address,
											vec![LinkUuid(transfer.uuid)],
											transfer.created_at,
										);

										ret.links.insert(link.clone());
										new_links.push(link);
									}

									// add to indexed data
									indexed_links.push(new_links);
								}
							}

							Ok::<_, ErrReport>((config_key, max_block_height, ret))
						}
					});
				}

				// don't process too many addresses at once
				if futures.len() >= MAX_ADDRESSES_PER_JOIN_SET {
					break;
				}
			}

			// collect results
			loop {
				tokio::select! {
					_ = networks_updated.changed() => {
						self.log(IndexType::Upstream, true, "Restarting… (networks updated)");
						break 'indexing Ok(());
					}
					result = futures.join_next() => {
						if let Some(res) = result {
							if let Ok((config_key, block_height, new_warehouse_data)) = res? {
								warehouse_data += new_warehouse_data;
								config_key_map.insert(config_key, block_height);
							}
						} else {
							break;
						}
					}
				}
			}

			// commit if collected enough
			if warehouse_data.should_commit(is_at_the_tip) && self.app.is_leading() {
				self.log(
					IndexType::Upstream,
					true,
					&format!(
						"Pushing {} record(s) to warehouse",
						style(self.format_number(warehouse_data.len())?).bold(),
					),
				);

				// push to warehouse
				warehouse_data.commit(self.app.warehouse.clone()).await?;

				// commit config marker updates
				Config::set_many::<_, BlockHeight>(self.app.db(), config_key_map.clone()).await?;
				config_key_map.clear();
			}

			// if no threads ever started, pause
			if is_at_the_tip {
				sleep(Duration::from_secs(1)).await;
			}
		}
	}

	async fn break_in_new_addresses(&self, network_ids: PrimaryIds) -> Result<()> {
		// get all newly added addresses for the provided networks
		let address_ids = Config::get_many_by_keywords::<_, PrimaryId>(
			self.app.db(),
			vec![format!("added_address")],
		)
		.await?
		.into_values()
		.map(|h| h.value)
		.collect::<Vec<PrimaryId>>();
		if !address_ids.is_empty() {
			let newly_added_addresses = Address::get_all_where(
				self.app.db(),
				Condition::all()
					.add(AddressColumn::NetworkId.is_in(network_ids))
					.add(AddressColumn::AddressId.is_in(address_ids)),
			)
			.await?;

			// delete all links that contain newly added entity addresses in the middle
			let mut address_map: HashMap<PrimaryId, HashSet<String>> = HashMap::new();
			for address in newly_added_addresses.clone().into_iter() {
				if let Some(set) = address_map.get_mut(&address.network_id) {
					set.insert(address.address);
				} else {
					address_map.insert(address.network_id, HashSet::from([address.address]));
				}
			}
			Link::delete_all_by_newly_added_addresses(&self.app.warehouse, address_map).await?;

			// delete configs for the newly added addresses
			Config::delete_many(
				self.app.db(),
				newly_added_addresses
					.iter()
					.map(|a| ConfigKey::NewlyAddedAddress(a.network_id, a.address_id))
					.collect(),
			)
			.await?;
		}

		Ok(())
	}
}
