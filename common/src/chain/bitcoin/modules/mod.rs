use async_trait::async_trait;
use bitcoin::blockdata::transaction::Transaction;
use eyre::Result;
use std::collections::HashMap;

use crate::{
	chain::{ModuleTrait, WarehouseData},
	BlockHeight,
};
pub use balance::BitcoinBalance;
pub use coinbase::BitcoinCoinbase;
pub use transfer::BitcoinTransfer;

mod balance;
mod coinbase;
mod transfer;

#[async_trait]
pub trait BitcoinModuleTrait: ModuleTrait + Send + Sync {
	async fn run(
		&self,
		block_height: BlockHeight,
		block_time: u32,
		tx: Transaction,
		inputs: HashMap<String, u64>,
		outputs: HashMap<String, u64>,
	) -> Result<WarehouseData>;
}
