use async_trait::async_trait;
use ethers::{
	abi::AbiEncode,
	types::{Transaction, TransactionReceipt},
	utils,
};
use eyre::Result;

use crate::{
	chain::{
		evm::{modules::EvmModuleTrait, EvmTopic},
		Evm, ModuleId, ModuleTrait, WarehouseData, U256,
	},
	models::{Amount, PrimaryId},
	BlockHeight,
};

pub struct EvmTokenBalance {
	network_id: PrimaryId,
}

impl ModuleTrait for EvmTokenBalance {
	fn new(network_id: PrimaryId) -> Self {
		Self { network_id }
	}

	fn get_id(&self) -> ModuleId {
		ModuleId::EvmTokenBalance
	}
}

#[async_trait]
impl EvmModuleTrait for EvmTokenBalance {
	async fn run(
		&self,
		evm: &Evm,
		block_height: BlockHeight,
		block_time: u32,
		tx: Transaction,
		receipt: TransactionReceipt,
	) -> Result<WarehouseData> {
		let mut ret = WarehouseData::new();

		for log in receipt.logs.into_iter() {
			// if log was removed, it's not valid
			if let Some(removed) = log.removed {
				if removed {
					continue;
				}
			}

			// process token `transfer` event
			match evm.get_topic(&log)? {
				EvmTopic::TokenTransfer(from, to, amount) if amount > U256::zero() => {
					ret.amounts.insert(Amount::new(
						self.get_id(),
						self.network_id,
						block_height,
						&tx.hash.encode_hex(),
						&utils::to_checksum(&from, None),
						Some(utils::to_checksum(&log.address, None)),
						U256::zero(),
						amount,
						block_time,
					));
					ret.amounts.insert(Amount::new(
						self.get_id(),
						self.network_id,
						block_height,
						&tx.hash.encode_hex(),
						&utils::to_checksum(&to, None),
						Some(utils::to_checksum(&log.address, None)),
						amount,
						U256::zero(),
						block_time,
					));
				}
				_ => {}
			}
		}

		Ok(ret)
	}
}
