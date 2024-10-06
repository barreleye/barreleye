use async_trait::async_trait;
use eyre::Result;
use std::collections::HashMap;

use crate::{
	chain::{
		bitcoin::{modules::BitcoinModuleTrait, schema::Transaction as ParquetTransaction},
		ModuleId, ModuleTrait, WarehouseData, U256,
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
		block_height: BlockHeight,
		block_time: u32,
		tx: ParquetTransaction,
		_inputs: HashMap<String, u64>,
		outputs: HashMap<String, u64>,
	) -> Result<WarehouseData> {
		let mut ret = WarehouseData::new();

		if tx.is_coinbase {
			let tx_hash = tx.hash.to_string();
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
