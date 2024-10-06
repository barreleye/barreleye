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
	models::{PrimaryId, Transfer},
	BlockHeight,
};

pub struct EvmTokenTransfer {
	network_id: PrimaryId,
}

impl ModuleTrait for EvmTokenTransfer {
	fn new(network_id: PrimaryId) -> Self {
		Self { network_id }
	}

	fn get_id(&self) -> ModuleId {
		ModuleId::EvmTokenTransfer
	}
}

#[async_trait]
impl EvmModuleTrait for EvmTokenTransfer {
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
					ret.transfers.insert(Transfer::new(
						self.get_id(),
						self.network_id,
						block_height,
						&tx.hash.encode_hex(),
						&utils::to_checksum(&from, None),
						&utils::to_checksum(&to, None),
						Some(utils::to_checksum(&log.address, None)),
						amount,
						amount,
						block_time,
					));
				}
				_ => {}
			}
		}

		Ok(ret)
	}
}
