use eyre::Result;
use sea_orm::ColumnTrait;
use std::{
	borrow::Cow,
	collections::{HashMap, HashSet},
	sync::Arc,
	time::SystemTime,
};
use tokio::{
	signal,
	sync::watch,
	task::JoinSet,
	time::{sleep, Duration},
};
use tracing::{info, span, trace, Level};
use uuid::Uuid;

use barreleye_common::{
	models::{
		Address, AddressColumn, Amount, Balance, Config, ConfigKey, Entity, Link, Network,
		NetworkColumn, PrimaryId, PrimaryIds, SoftDeleteModel, Transfer,
	},
	utils, App, AppError, BlockHeight, INDEXER_HEARTBEAT_INTERVAL, INDEXER_PROMOTION_TIMEOUT,
};

mod link;
mod process;
mod sync;

#[derive(Clone)]
pub struct Indexer {
	app: Arc<App>,
}

impl Indexer {
	pub fn new(app: Arc<App>) -> Self {
		let span = span!(Level::INFO, "indexer");
		let _enter = span.enter();

		info!("started…");

		Self { app }
	}

	pub async fn start(&self) -> Result<()> {
		loop {
			self.prune_data().await?;

			let mut set = JoinSet::new();
			let (tx, rx) = watch::channel(SystemTime::now());

			set.spawn({
				let s = self.clone();
				let r = rx.clone();
				async move { s.sync(r).await }
			});

			set.spawn({
				let s = self.clone();
				let r = rx.clone();
				async move { s.process(r).await }
			});

			set.spawn({
				let s = self.clone();
				let r = rx.clone();
				async move { s.link(r).await }
			});

			let ret = tokio::select! {
				_ = signal::ctrl_c() => break Ok(()),
				v = self.primary_check() => v,
				v = self.networks_check(tx) => v,
				v = self.show_progress() => v,
				v = async {
					while let Some(res) = set.join_next().await {
						res??;
					}

					Ok(())
				} => v,
			};

			if let Err(err) = ret {
				return Err(AppError::Indexing { error: Cow::Owned(err.to_string()) }.into());
			}
		}
	}

	async fn primary_check(&self) -> Result<()> {
		let db = self.app.db();
		let uuid = self.app.uuid;

		loop {
			let cool_down_period = utils::ago_in_seconds(INDEXER_PROMOTION_TIMEOUT / 2);

			let last_primary = Config::get::<_, Uuid>(db, ConfigKey::Primary).await?;
			match last_primary {
				None => {
					// first run ever
					Config::set::<_, Uuid>(db, ConfigKey::Primary, uuid).await?;
				}
				Some(hit) if hit.value == uuid && hit.updated_at >= cool_down_period => {
					// if primary, check-in only if cool-down period has not
					// started yet ↑
					if Config::set_where::<_, Uuid>(db, ConfigKey::Primary, uuid, hit).await? {
						self.app.set_is_primary(true).await?;
					}
				}
				Some(hit) if utils::ago_in_seconds(INDEXER_PROMOTION_TIMEOUT) > hit.updated_at => {
					// attempt to upgrade to primary (set is_primary on the next
					// iteration)
					Config::set_where::<_, Uuid>(db, ConfigKey::Primary, uuid, hit).await?;
				}
				_ => {
					// either cool-down period has started or this is a
					// secondary
					self.app.set_is_primary(false).await?;
				}
			}

			sleep(Duration::from_secs(INDEXER_HEARTBEAT_INTERVAL)).await
		}
	}

	async fn networks_check(&self, tx: watch::Sender<SystemTime>) -> Result<()> {
		let mut networks_updated_at =
			Config::get::<_, u8>(self.app.db(), ConfigKey::NetworksUpdated)
				.await?
				.map(|v| v.updated_at)
				.unwrap_or_else(utils::now);

		loop {
			match Config::get::<_, u8>(self.app.db(), ConfigKey::NetworksUpdated).await? {
				Some(value) if value.updated_at != networks_updated_at => {
					networks_updated_at = value.updated_at;
					tx.send(SystemTime::now())?;
				}
				_ => {}
			}

			sleep(Duration::from_secs(1)).await;
		}
	}

	#[tracing::instrument(name = "indexer", skip_all)]
	async fn show_progress(&self) -> Result<()> {
		let mut started_indexing = false;

		loop {
			if !self.app.is_leading() {
				if started_indexing {
					trace!("stopping…");
				}

				started_indexing = false;
				sleep(Duration::from_secs(1)).await;
				continue;
			}

			if !started_indexing {
				started_indexing = true;

				trace!("starting…");
			}

			if self.app.networks.read().await.is_empty() {
				trace!(message = "no active networks…", rechecking = "10s");
				sleep(Duration::from_secs(10)).await;
				continue;
			}

			sleep(Duration::from_secs(5)).await;
		}
	}

	async fn prune_data(&self) -> Result<()> {
		// prune all soft-deleted addresses
		let addresses = Address::get_all_deleted(self.app.db()).await?;
		if !addresses.is_empty() {
			// delete all upstream configs
			Config::delete_many(
				self.app.db(),
				addresses
					.iter()
					.map(|a| ConfigKey::IndexerLink(a.network_id, a.address_id))
					.collect(),
			)
			.await?;

			// delete all addresses
			Address::prune_all_where(
				self.app.db(),
				AddressColumn::AddressId.is_in(Into::<PrimaryIds>::into(addresses.clone())),
			)
			.await?;

			// delete links from warehouse
			let mut sources: HashMap<PrimaryId, HashSet<String>> = HashMap::new();
			for address in addresses.into_iter() {
				if let Some(set) = sources.get_mut(&address.network_id) {
					set.insert(address.address);
				} else {
					sources.insert(address.network_id, HashSet::from([address.address]));
				}
			}
			Link::delete_all_by_sources(&self.app.warehouse, sources).await?;
		}

		// prune all soft-deleted entities
		Entity::prune_all(self.app.db()).await?;

		// prune all soft-deleted networks
		let deleted_networks = Network::get_all_existing(self.app.db(), Some(true)).await?;
		if !deleted_networks.is_empty() {
			let network_ids: PrimaryIds = deleted_networks.clone().into();

			// delete all associated configs
			Config::delete_all_by_keywords(
				self.app.db(),
				deleted_networks.clone().iter().map(|n| format!("n{}", n.network_id)).collect(),
			)
			.await?;

			// delete all addresses
			Address::prune_all_where(
				self.app.db(),
				AddressColumn::NetworkId.is_in(network_ids.clone()),
			)
			.await?;

			// delete from warehouse
			let (transfers_deleted, balances_deleted, amounts_deleted, links_deleted) = tokio::join!(
				Transfer::delete_all_by_network_id(&self.app.warehouse, network_ids.clone()),
				Balance::delete_all_by_network_id(&self.app.warehouse, network_ids.clone()),
				Amount::delete_all_by_network_id(&self.app.warehouse, network_ids.clone()),
				Link::delete_all_by_network_id(&self.app.warehouse, network_ids.clone()),
			);

			transfers_deleted.and(balances_deleted).and(amounts_deleted).and(links_deleted)?;

			// finally delete only the networks we grabbed earlier
			Network::prune_all_where(self.app.db(), NetworkColumn::NetworkId.is_in(network_ids))
				.await?;
		}

		Ok(())
	}

	async fn get_updated_block_height(
		&self,
		network_id: PrimaryId,
		last_known_block_height: Option<BlockHeight>,
	) -> Result<BlockHeight> {
		let mut ret = 0;

		if let Some(chain) = self.app.networks.read().await.get(&network_id) {
			let config_key = ConfigKey::BlockHeight(network_id);
			ret = match Config::get::<_, BlockHeight>(self.app.db(), config_key).await? {
				Some(hit) if hit.value > last_known_block_height.unwrap_or(0) => hit.value,
				_ => {
					let block_height = chain.get_block_height().await?;

					Config::set::<_, BlockHeight>(self.app.db(), config_key, block_height).await?;

					block_height
				}
			};
		}

		Ok(ret)
	}

	fn get_block_chunk_ranges(
		&self,
		block_height: BlockHeight,
	) -> Result<Vec<(BlockHeight, BlockHeight)>> {
		let mut ret = vec![];

		let chunks = self.app.cpu_count - 1; // always leave 1 for tail sync
		let chunk_size = ((block_height - 1) as f64 / chunks as f64).floor() as u64;

		let mut block_height_min = 0;
		let mut block_height_max = chunk_size;

		for i in 0..chunks {
			if i + 1 == chunks {
				block_height_max = block_height - 1
			}

			ret.push((block_height_min, block_height_max));

			block_height_min = block_height_max;
			block_height_max += chunk_size;
		}

		Ok(ret)
	}
}
