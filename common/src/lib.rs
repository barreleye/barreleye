use chrono::NaiveDateTime;
use clap::{builder::PossibleValue, ValueEnum};
use derive_more::Display;
use eyre::{Report, Result};
use futures::{stream::FuturesUnordered, StreamExt};
use governor::{
	clock::DefaultClock,
	state::{direct::NotKeyed, InMemoryState},
	RateLimiter as GovernorRateLimiter,
};
use sea_orm::{entity::prelude::*, DatabaseTransaction, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	fmt::Debug,
	process,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::{
	chain::{Bitcoin, BoxedChain, Evm},
	models::{Config, ConfigKey, Network, PrimaryId, SoftDeleteModel},
};
pub use db::Db;
pub use errors::AppError;
pub use s3::{Service as S3Service, S3};
pub use settings::Settings;
pub use storage::Storage;
pub use warehouse::Warehouse;

pub mod chain;
pub mod db;
pub mod errors;
pub mod models;
pub mod s3;
pub mod settings;
pub mod storage;
pub mod utils;
pub mod warehouse;

mod banner;

pub const INDEXER_PROMOTION_TIMEOUT: u64 = 20;
pub const INDEXER_HEARTBEAT_INTERVAL: u64 = 2;

pub type BlockHeight = u64;
pub type RateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

#[derive(Clone)]
pub struct App {
	pub uuid: Uuid,
	pub networks: Arc<RwLock<HashMap<PrimaryId, Arc<BoxedChain>>>>,
	pub settings: Arc<Settings>,
	pub storage: Arc<Storage>,
	db: Arc<Db>,
	pub warehouse: Arc<Warehouse>,
	is_ready: Arc<AtomicBool>,
	is_primary: Arc<AtomicBool>,
	connected_at: Arc<RwLock<Option<NaiveDateTime>>>,
	pub cpu_count: usize,
}

impl App {
	pub async fn new(
		settings: Arc<Settings>,
		storage: Arc<Storage>,
		db: Arc<Db>,
		warehouse: Arc<Warehouse>,
	) -> Result<Self> {
		let mut app = App {
			uuid: utils::new_uuid(),
			networks: Arc::new(RwLock::new(HashMap::new())),
			settings: settings.clone(),
			storage,
			db,
			warehouse,
			is_ready: Arc::new(AtomicBool::new(false)),
			is_primary: Arc::new(AtomicBool::new(false)),
			connected_at: Arc::new(RwLock::new(None)),
			cpu_count: num_cpus::get(),
		};

		let networks = app.get_networks().await?;

		// check networks and log errors
		if networks.is_empty() {
			if settings.is_indexer {
				warn!("no active networks found (add networks via API)");
			}
		} else {
			networks
				.values()
				.filter(|chain| settings.is_indexer && chain.get_network().rps == 0)
				.for_each(|chain| {
					warn!("{} rpc requests are not rate-limited", chain.get_network().name);
				});
		}

		app.networks = Arc::new(RwLock::new(networks));

		Ok(app)
	}

	pub fn db(&self) -> &DatabaseConnection {
		self.db.get()
	}

	pub async fn db_tx(&self) -> Result<DatabaseTransaction> {
		Ok(self.db().begin().await?)
	}

	pub async fn get_networks(&self) -> Result<HashMap<PrimaryId, Arc<BoxedChain>>> {
		let mut ret = HashMap::new();

		for n in Network::get_all_existing(self.db(), Some(false)).await?.into_iter() {
			let network_id = n.network_id;

			let boxed_chain: BoxedChain = match n.architecture {
				Architecture::Bitcoin => Box::new(Bitcoin::new(n)),
				Architecture::Evm => Box::new(Evm::new(n)),
			};

			ret.insert(network_id, Arc::new(boxed_chain));
		}

		Ok(ret)
	}

	pub async fn should_reconnect(&self) -> Result<bool> {
		Ok(match *self.connected_at.read().await {
			Some(connected_at) => {
				let networks_updated_at =
					Config::get::<_, u8>(self.db(), ConfigKey::NetworksUpdated)
						.await?
						.map(|v| v.updated_at)
						.unwrap_or_else(utils::now);

				connected_at < networks_updated_at
			}
			None => true,
		})
	}

	pub async fn connect_networks(&self, silent: bool) -> Result<()> {
		let mut futures = FuturesUnordered::new();
		let networks = Network::get_all_existing(self.db(), Some(false)).await?;

		for n in networks {
			futures.push(async move {
				if !silent {
					info!("connecting to {} ({})…", n.name, n.id);
				}

				let mut boxed_chain: BoxedChain = match n.architecture {
					Architecture::Bitcoin => Box::new(Bitcoin::new(n.clone())),
					Architecture::Evm => Box::new(Evm::new(n.clone())),
				};

				if boxed_chain.connect().await? {
					Ok(Arc::new(boxed_chain))
				} else {
					if !silent {
						warn!("could not connect to {} ({})", n.name, n.id);
					}
					Err(Report::msg(format!(
						"could not connect to an rpc endpoint for {} ({})",
						n.name, n.id
					)))
				}
			});
		}

		let mut connected_networks = HashMap::new();
		let mut failures = Vec::new();

		while let Some(result) = futures.next().await {
			match result {
				Ok(chain) => {
					let network_id = chain.get_network().network_id;
					connected_networks.insert(network_id, chain);
				}
				Err(e) => failures.push(e.to_string()),
			}
		}

		if !failures.is_empty() {
			return Err(Report::msg(failures.join("\n")));
		}

		let mut networks = self.networks.write().await;
		*networks = connected_networks;

		let mut connected_at = self.connected_at.write().await;
		*connected_at = Some(utils::now());

		Ok(())
	}

	pub fn is_leading(&self) -> bool {
		self.is_ready() && self.is_primary()
	}

	pub fn is_ready(&self) -> bool {
		self.is_ready.load(Ordering::SeqCst)
	}

	pub fn set_is_ready(&self) {
		self.is_ready.store(true, Ordering::SeqCst);
	}

	pub fn is_primary(&self) -> bool {
		self.is_primary.load(Ordering::SeqCst)
	}

	pub async fn set_is_primary(&self, is_primary: bool) -> Result<()> {
		if is_primary != self.is_primary() {
			self.is_primary.store(is_primary, Ordering::SeqCst);
		}

		Ok(())
	}

	pub async fn format_address(&self, address: &str) -> Result<String> {
		for (_, chain) in self.networks.read().await.iter() {
			let formatted_address = chain.format_address(address);
			if formatted_address != address {
				return Ok(formatted_address);
			}
		}

		Ok(address.to_string())
	}
}

#[derive(Clone, Display, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum IdPrefix {
	#[display("net")]
	Network,
	#[display("key")]
	ApiKey,
	#[display("ent")]
	Entity,
	#[display("adr")]
	Address,
	#[display("tag")]
	Tag,
	#[display("tok")]
	Token,
}

#[derive(
	Default,
	Debug,
	DeriveActiveEnum,
	Copy,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Serialize,
	Deserialize,
)]
#[sea_orm(rs_type = "i16", db_type = "SmallInteger")]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
	#[default]
	Low = 1,
	High = 2,
	Critical = 3,
}

// @TODO for some reason `EnumIter` in sea-orm v1.0.0 doesn't work
impl strum::IntoEnumIterator for RiskLevel {
	type Iterator = std::array::IntoIter<RiskLevel, 3>;

	fn iter() -> Self::Iterator {
		[RiskLevel::Low, RiskLevel::High, RiskLevel::Critical].into_iter()
	}
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum RiskReason {
	Entity,
	Source,
}

#[derive(Default, Debug, DeriveActiveEnum, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[sea_orm(rs_type = "i16", db_type = "SmallInteger")]
#[serde(rename_all = "camelCase")]
pub enum Architecture {
	#[default]
	Bitcoin = 1,
	Evm = 2,
}

// @TODO for some reason `EnumIter` in sea-orm v1.0.0 doesn't work
impl strum::IntoEnumIterator for Architecture {
	type Iterator = std::array::IntoIter<Architecture, 2>;

	fn iter() -> Self::Iterator {
		[Architecture::Bitcoin, Architecture::Evm].into_iter()
	}
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
	#[default]
	Both,
	Indexer,
	Http,
}

impl ValueEnum for Mode {
	fn value_variants<'a>() -> &'a [Self] {
		&[Self::Both, Self::Indexer, Self::Http]
	}

	fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
		match self {
			Self::Both => Some(PossibleValue::new("both")),
			Self::Indexer => Some(PossibleValue::new("indexer")),
			Self::Http => Some(PossibleValue::new("http")),
		}
	}
}

pub fn quit(app_error: AppError) -> ! {
	error!("{}", app_error.to_string());

	process::exit(match app_error {
		AppError::SignalHandler | AppError::ServerStartup { .. } => exitcode::OSERR,
		AppError::Config { .. } => exitcode::CONFIG,
		_ => exitcode::UNAVAILABLE,
	})
}
