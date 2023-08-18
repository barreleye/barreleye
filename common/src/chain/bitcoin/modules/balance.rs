use async_trait::async_trait;
use bitcoin::blockdata::transaction::Transaction;
use eyre::Result;
use std::collections::HashMap;

use crate::{
	chain::{bitcoin::modules::BitcoinModuleTrait, ModuleId, ModuleTrait, WarehouseData, U256},
	models::{Amount, PrimaryId},
	BlockHeight,
};

pub struct BitcoinBalance {
	network_id: PrimaryId,
}

impl ModuleTrait for BitcoinBalance {
	fn new(network_id: PrimaryId) -> Self {
		Self { network_id }
	}

	fn get_id(&self) -> ModuleId {
		ModuleId::BitcoinBalance
	}
}

#[async_trait]
impl BitcoinModuleTrait for BitcoinBalance {
	async fn run(
		&self,
		block_height: BlockHeight,
		block_time: u32,
		tx: Transaction,
		inputs: HashMap<String, u64>,
		outputs: HashMap<String, u64>,
	) -> Result<WarehouseData> {
		let mut ret = WarehouseData::new();
		let mut balance_map = HashMap::<String, (u64, u64)>::new();

		if !tx.is_coin_base() {
			for (address, new_amount) in inputs.into_iter() {
				if let Some(amounts) = balance_map.get_mut(&address) {
					amounts.1 += new_amount;
				} else {
					balance_map.insert(address, (0, new_amount));
				}
			}
		}

		for (address, new_amount) in outputs.into_iter() {
			if let Some(amounts) = balance_map.get_mut(&address) {
				amounts.0 += new_amount;
			} else {
				balance_map.insert(address, (new_amount, 0));
			}
		}

		let tx_hash = tx.txid().as_raw_hash().to_string();

		for (address, (amount_in, amount_out)) in balance_map.into_iter() {
			ret.amounts.insert(Amount::new(
				self.get_id(),
				self.network_id,
				block_height,
				&tx_hash.clone(),
				&address,
				None,
				U256::from_str_radix(&amount_in.to_string(), 10)?,
				U256::from_str_radix(&amount_out.to_string(), 10)?,
				block_time,
			));
		}

		Ok(ret)
	}
}
