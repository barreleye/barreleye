use async_trait::async_trait;
use bitcoin::{
	address::Address, blockdata::transaction::Transaction, hash_types::Txid,
	Network as BitcoinNetwork,
};
use eyre::Result;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use url::Url;

use crate::{
	chain::{ChainTrait, ModuleId, ModuleTrait, WarehouseData},
	models::Network,
	utils, BlockHeight, RateLimiter, Storage,
};
use client::{Auth, Client};
use modules::{BitcoinBalance, BitcoinCoinbase, BitcoinModuleTrait, BitcoinTransfer};
use schema::{
	Block as ParquetBlock, Input as ParquetInput, Output as ParquetOutput, ParquetFile,
	Transaction as ParquetTransaction,
};

mod client;
mod modules;
mod schema;

pub struct Bitcoin {
	network: Network,
	rpc: Option<String>,
	client: Option<Arc<Client>>,
	bitcoin_network: BitcoinNetwork,
	rate_limiter: Option<Arc<RateLimiter>>,
	modules: Vec<Box<dyn BitcoinModuleTrait>>,
}

impl Bitcoin {
	pub fn new(network: Network) -> Self {
		let rps = network.rps as u32;
		let network_id = network.network_id;

		Self {
			network,
			rpc: None,
			client: None,
			bitcoin_network: BitcoinNetwork::Bitcoin,
			rate_limiter: utils::get_rate_limiter(rps),
			modules: vec![
				Box::new(BitcoinTransfer::new(network_id)),
				Box::new(BitcoinBalance::new(network_id)),
				Box::new(BitcoinCoinbase::new(network_id)),
			],
		}
	}
}

#[async_trait]
impl ChainTrait for Bitcoin {
	async fn connect(&mut self) -> Result<bool> {
		if let Ok(u) = Url::parse(&self.network.rpc_endpoint) {
			let auth = match (u.username(), u.password()) {
				(username, Some(password)) => {
					Auth::UserPass(username.to_string(), password.to_string())
				}
				_ => Auth::None,
			};

			if let Some(rate_limiter) = &self.rate_limiter {
				rate_limiter.until_ready().await;
			}

			let client = Client::new_without_retry(&self.network.rpc_endpoint, auth.clone());
			if client.get_blockchain_info().await.is_ok() {
				self.client = Some(Arc::new(Client::new(&self.network.rpc_endpoint, auth)));
				self.rpc = Some(self.network.rpc_endpoint.clone());
			}
		}

		Ok(self.is_connected())
	}

	fn is_connected(&self) -> bool {
		self.client.is_some()
	}

	fn get_network(&self) -> Network {
		self.network.clone()
	}

	fn get_rpc(&self) -> Option<String> {
		self.rpc.clone()
	}

	fn get_module_ids(&self) -> Vec<ModuleId> {
		self.modules.iter().map(|m| m.get_id()).collect()
	}

	fn get_rate_limiter(&self) -> Option<Arc<RateLimiter>> {
		self.rate_limiter.clone()
	}

	fn format_address(&self, address: &str) -> String {
		if let Ok(unknown_address) = Address::from_str(address) {
			if let Ok(parsed_address) = unknown_address.require_network(self.bitcoin_network) {
				return parsed_address.to_string();
			}
		}

		address.to_string()
	}

	async fn get_block_height(&self) -> Result<BlockHeight> {
		self.rate_limit().await;
		Ok(self.client.as_ref().unwrap().get_block_count().await?)
	}

	async fn process_block(
		&self,
		block_height: BlockHeight,
		module_ids: Vec<ModuleId>,
	) -> Result<Option<WarehouseData>> {
		let mut ret = None;

		self.rate_limit().await;
		if let Ok(block_hash) = self.client.as_ref().unwrap().get_block_hash(block_height).await {
			self.rate_limit().await;
			if let Ok(block) = self.client.as_ref().unwrap().get_block(&block_hash).await {
				let mut warehouse_data = WarehouseData::new();

				for tx in block.txdata.into_iter() {
					warehouse_data += self
						.process_transaction(
							block_height,
							block.header.time,
							tx,
							module_ids.clone(),
						)
						.await?;
				}

				ret = Some(warehouse_data);
			}
		}

		Ok(ret)
	}

	async fn extract_block(
		&self,
		storage: Arc<Storage>,
		block_height: BlockHeight,
	) -> Result<bool> {
		let storage_db = storage.get(self.network.network_id, block_height)?;

		self.rate_limit().await;
		if let Ok(block_hash) = self.client.as_ref().unwrap().get_block_hash(block_height).await {
			self.rate_limit().await;
			if let Ok(block) = self.client.as_ref().unwrap().get_block(&block_hash).await {
				storage_db.insert(ParquetBlock {
					hash: block_hash,
					version: block.header.version.to_consensus(),
					prev_blockhash: block.header.prev_blockhash,
					merkle_root: block.header.merkle_root,
					time: block.header.time,
					bits: block.header.bits.to_consensus(),
					nonce: block.header.nonce,
				})?;

				for tx in block.txdata.into_iter() {
					storage_db.insert(ParquetTransaction {
						hash: *tx.txid().as_raw_hash(),
						version: tx.version,
						lock_time: tx.lock_time.to_consensus_u32(),
						inputs: tx.input.len() as u32,
						outputs: tx.output.len() as u32,
					})?;

					for txin in tx.input.into_iter() {
						storage_db.insert(ParquetInput {
							previous_output_tx_hash: txin
								.previous_output
								.txid
								.as_raw_hash()
								.to_string(),
							previous_output_vout: txin.previous_output.vout,
						})?;
					}

					for txout in tx.output.into_iter() {
						storage_db.insert(ParquetOutput {
							value: txout.value,
							script_pubkey: txout.script_pubkey.to_string(),
						})?;
					}
				}
			}
		}

		storage_db.commit(vec![
			ParquetFile::Block.to_string(),
			ParquetFile::Transactions.to_string(),
			ParquetFile::Inputs.to_string(),
			ParquetFile::Outputs.to_string(),
		])?;

		Ok(true)
	}
}

impl Bitcoin {
	async fn process_transaction(
		&self,
		block_height: BlockHeight,
		block_time: u32,
		tx: Transaction,
		module_ids: Vec<ModuleId>,
	) -> Result<WarehouseData> {
		let mut ret = WarehouseData::new();

		let get_unique_addresses = move |pair: Vec<(String, u64)>| {
			let mut m = HashMap::<String, u64>::new();

			for p in pair.into_iter() {
				let (address, value) = p;
				let address_key = address.to_string();

				let initial_value = m.get(&address_key).unwrap_or(&0);
				m.insert(address_key, *initial_value + value);
			}

			m
		};

		let inputs = get_unique_addresses({
			let mut ret = vec![];

			for txin in tx.input.iter() {
				let (txid, vout) = (txin.previous_output.txid, txin.previous_output.vout);

				// if !txid.is_empty() && !tx.is_coin_base() {
				if !tx.is_coin_base() {
					if let Some((a, v)) = self.get_utxo(txid, vout).await? {
						ret.push((a, v))
					}
				}
			}

			ret
		});

		let outputs = get_unique_addresses(self.index_transaction_outputs(&tx).await?);

		for module in self.modules.iter().filter(|m| module_ids.contains(&m.get_id())) {
			ret += module
				.run(self, block_height, block_time, tx.clone(), inputs.clone(), outputs.clone())
				.await?;
		}

		Ok(ret)
	}

	async fn index_transaction_outputs(&self, tx: &Transaction) -> Result<Vec<(String, u64)>> {
		let mut ret = vec![];

		for (i, txout) in tx.output.iter().enumerate() {
			if let Some(address) = self.get_address(tx, i as u32)? {
				ret.push((address, txout.value));
			}
		}

		Ok(ret)
	}

	async fn get_utxo(&self, txid: Txid, vout: u32) -> Result<Option<(String, u64)>> {
		// "-txindex" is a requirement for bitcoin, because this project does not use caching
		// and we therefore have to rely on bitcoin's internal indexing for "(network_id, txid) -> block_height"
		self.rate_limit().await;
		let tx = self.client.as_ref().unwrap().get_raw_transaction(&txid, None).await?;
		let ret = self.get_address(&tx, vout)?.map(|a| {
			let v = tx.output[vout as usize].value;
			(a, v)
		});

		Ok(ret)
	}

	fn get_address(&self, tx: &Transaction, vout: u32) -> Result<Option<String>> {
		let mut ret = None;

		if vout < tx.output.len() as u32 {
			if let Ok(address) =
				Address::from_script(&tx.output[vout as usize].script_pubkey, self.bitcoin_network)
			{
				ret = Some(address.to_string());
			} else {
				ret = Some(format!("{}:{}", tx.txid().as_raw_hash(), vout));
			}
		}

		Ok(ret)
	}

	fn _is_valid_address(&self, address: &str) -> bool {
		!address.contains(':')
	}
}
