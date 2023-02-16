use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_more::Display;
use eyre::Result;
use std::{collections::HashSet, ops::AddAssign, sync::Arc};
use tokio::task::JoinSet;

pub use crate::chain::bitcoin::Bitcoin;
use crate::{
	models::{Amount, Link, Network, Relation, Transfer},
	utils, BlockHeight, PrimaryId, RateLimiter, Warehouse,
};
pub use evm::Evm;
pub use u256::U256;

pub mod bitcoin;
pub mod evm;
pub mod u256;

pub type BoxedChain = Box<dyn ChainTrait>;

#[repr(u16)]
#[derive(Display, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ModuleId {
	BitcoinCoinbase = 101,
	BitcoinTransfer = 102,
	BitcoinBalance = 103,
	BitcoinRelationBalanceTransfer = 104,
	BitcoinRelationNoChange = 105,
	EvmTransfer = 201,
	EvmBalance = 202,
	EvmTokenTransfer = 203,
	EvmTokenBalance = 204,
}

#[async_trait]
pub trait ChainTrait: Send + Sync {
	async fn connect(&mut self) -> Result<bool>;
	fn is_connected(&self) -> bool;

	fn get_network(&self) -> Network;
	fn get_rpc(&self) -> Option<String>;
	fn get_module_ids(&self) -> Vec<ModuleId>;
	fn format_address(&self, address: &str) -> String;
	fn get_rate_limiter(&self) -> Option<Arc<RateLimiter>>;

	async fn get_block_height(&self) -> Result<BlockHeight>;

	async fn process_block(
		&self,
		block_height: BlockHeight,
		modules: Vec<ModuleId>,
	) -> Result<Option<WarehouseData>>;

	async fn extract_block(&self, block_height: BlockHeight) -> Result<bool>;

	async fn rate_limit(&self) {
		if let Some(rate_limiter) = &self.get_rate_limiter() {
			rate_limiter.until_ready().await;
		}
	}
}

#[async_trait]
pub trait ModuleTrait {
	fn new(network_id: PrimaryId) -> Self
	where
		Self: Sized;
	fn get_id(&self) -> ModuleId;
}

#[derive(Debug, Default, Clone)]
pub struct WarehouseData {
	saved_at: NaiveDateTime,
	pub transfers: HashSet<Transfer>,
	pub amounts: HashSet<Amount>,
	pub relations: HashSet<Relation>,
	pub links: HashSet<Link>,
}

impl WarehouseData {
	pub fn new() -> Self {
		Self { saved_at: utils::now(), ..Default::default() }
	}

	pub fn len(&self) -> usize {
		self.transfers.len() + self.amounts.len() + self.relations.len() + self.links.len()
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	pub fn should_commit(&self, force: bool) -> bool {
		let (min_secs, max_secs) = (1, 10);

		let manually_required = force && !self.is_empty();
		let lengthy_break = utils::ago_in_seconds(max_secs) > self.saved_at && !self.is_empty();
		let buffer_is_full = utils::ago_in_seconds(min_secs) > self.saved_at && self.len() > 50_000;

		manually_required || lengthy_break || buffer_is_full
	}

	pub async fn commit(&mut self, warehouse: Arc<Warehouse>) -> Result<()> {
		let mut set = JoinSet::new();

		if !self.transfers.is_empty() {
			set.spawn({
				let w = warehouse.clone();
				let t = self.transfers.clone().into_iter().collect();
				async move { Transfer::create_many(&w, t).await }
			});
		}
		if !self.amounts.is_empty() {
			set.spawn({
				let w = warehouse.clone();
				let a = self.amounts.clone().into_iter().collect();
				async move { Amount::create_many(&w, a).await }
			});
		}
		if !self.relations.is_empty() {
			set.spawn({
				let w = warehouse.clone();
				let r = self.relations.clone().into_iter().collect();
				async move { Relation::create_many(&w, r).await }
			});
		}
		if !self.links.is_empty() {
			set.spawn({
				let w = warehouse.clone();
				let l = self.links.clone().into_iter().collect();
				async move { Link::create_many(&w, l).await }
			});
		}

		while let Some(res) = set.join_next().await {
			res??;
		}

		self.clear();
		Ok(())
	}

	pub fn clear(&mut self) {
		self.saved_at = utils::now();

		self.transfers.clear();
		self.amounts.clear();
		self.relations.clear();
		self.links.clear();
	}
}

impl AddAssign for WarehouseData {
	fn add_assign(&mut self, rhs: WarehouseData) {
		self.transfers.extend(rhs.transfers);
		self.amounts.extend(rhs.amounts);
		self.relations.extend(rhs.relations);
		self.links.extend(rhs.links);
	}
}
