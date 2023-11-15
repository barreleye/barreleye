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

pub struct BitcoinTransfer {
	network_id: PrimaryId,
}

impl ModuleTrait for BitcoinTransfer {
	fn new(network_id: PrimaryId) -> Self {
		Self { network_id }
	}

	fn get_id(&self) -> ModuleId {
		ModuleId::BitcoinTransfer
	}
}

#[async_trait]
impl BitcoinModuleTrait for BitcoinTransfer {
	async fn run(
		&self,
		block_height: BlockHeight,
		block_time: u32,
		tx: ParquetTransaction,
		inputs: HashMap<String, u64>,
		outputs: HashMap<String, u64>,
	) -> Result<WarehouseData> {
		let mut ret = WarehouseData::new();

		if tx.is_coinbase {
			return Ok(ret);
		}

		let tx_hash = tx.hash.to_string();
		let input_amount_total: u64 = inputs.clone().into_values().sum();
		let output_amount_total: u64 = outputs.clone().into_values().sum();
		let batch_amount = U256::from_str_radix(&output_amount_total.to_string(), 10)?;

		for input in inputs.iter() {
			for output in outputs.iter() {
				let (from, to) = (input.0.clone(), output.0.clone());
				if from != to {
					let amount = match input_amount_total > 0 {
						true => ((*input.1 as f64 / input_amount_total as f64) * *output.1 as f64)
							.round(),
						_ => 0.0,
					};

					ret.transfers.insert(Transfer::new(
						self.get_id(),
						self.network_id,
						block_height,
						&tx_hash.clone(),
						&from,
						&to,
						None,
						U256::from_str_radix(&amount.to_string(), 10)?,
						batch_amount,
						block_time,
					));
				}
			}
		}

		Ok(ret)
	}
}
