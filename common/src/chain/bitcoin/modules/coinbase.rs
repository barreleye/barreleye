use async_trait::async_trait;
use bitcoin::blockdata::transaction::Transaction;
use eyre::Result;
use std::collections::HashMap;

use crate::{
	chain::{
		bitcoin::modules::BitcoinModuleTrait, Bitcoin, ModuleId, ModuleTrait, WarehouseData, U256,
	},
	models::{PrimaryId, Transfer},
	BlockHeight,
};

pub struct BitcoinCoinbase {
	network_id: PrimaryId,
}

impl ModuleTrait for BitcoinCoinbase {
	fn new(network_id: PrimaryId) -> Self {
		Self { network_id }
	}

	fn get_id(&self) -> ModuleId {
		ModuleId::BitcoinCoinbase
	}
}

#[async_trait]
impl BitcoinModuleTrait for BitcoinCoinbase {
	async fn run(
		&self,
		_bitcoin: &Bitcoin,
		block_height: BlockHeight,
		block_time: u32,
		tx: Transaction,
		_inputs: HashMap<String, u64>,
		outputs: HashMap<String, u64>,
	) -> Result<WarehouseData> {
		let mut ret = WarehouseData::new();

		if tx.is_coin_base() {
			let tx_hash = tx.txid().as_raw_hash().to_string();
			let output_amount_total: u64 = outputs.clone().into_values().sum();
			let batch_amount = U256::from_str_radix(&output_amount_total.to_string(), 10)?;

			for (to, amount) in outputs.into_iter() {
				ret.transfers.insert(Transfer::new(
					self.get_id(),
					self.network_id,
					block_height,
					&tx_hash.clone(),
					"",
					&to,
					None,
					U256::from_str_radix(&amount.to_string(), 10)?,
					batch_amount,
					block_time,
				));
			}
		}

		Ok(ret)
	}
}
