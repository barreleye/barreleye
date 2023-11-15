use base64::{engine::general_purpose, Engine as _};
use bitcoin::{Block, BlockHash};
use bitcoincore_rpc_json::{
	bitcoin::{
		consensus::{Decodable, ReadExt},
		hashes::hex::HexToBytesIter,
	},
	GetBlockchainInfoResult,
};
use derive_more::{Display, Error};
use eyre::{eyre, Result};
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{sleep, Duration};

// source: `https://github.com/bitcoin/bitcoin/blob/master/src/rpc/protocol.h`
const RPC_IN_WARMUP: i32 = -28;

const RETRY_ATTEMPTS: u32 = 13;
const RPC_TIMEOUT: u64 = 250;

#[derive(Debug, Display, Error)]
pub enum ClientError {
	#[display(fmt = "{message}")]
	General { message: String },
	#[display(fmt = "Could not connect to rpc endpoint")]
	Connection,
	#[display(fmt = "RPC error: {message}")]
	Rpc { message: String },
	#[display(fmt = "Nonce mismatch")]
	NonceMismatch,
}

#[derive(Clone)]
pub enum Auth {
	None,
	UserPass(String, String),
}

#[derive(Debug, Deserialize)]
struct RpcError {
	code: i32,
	message: String,
}

#[derive(Debug, Deserialize)]
struct Response {
	result: JsonValue,
	error: Option<RpcError>,
	id: Option<String>,
}

// @NOTE using custom client because `bitcoincore-rpc@0.16.0` is not async + doesn't support https
pub struct Client {
	url: String,
	auth: Auth,
	id: AtomicUsize,
	with_retry: bool,
}

impl Client {
	pub fn new(url: &str, auth: Auth) -> Self {
		Self { url: url.to_string(), auth, id: AtomicUsize::new(1), with_retry: true }
	}

	pub fn new_without_retry(url: &str, auth: Auth) -> Self {
		Self { url: url.to_string(), auth, id: AtomicUsize::new(1), with_retry: false }
	}

	pub async fn get_blockchain_info(&self) -> Result<GetBlockchainInfoResult> {
		let result = self.request("getblockchaininfo", &[]).await?;
		Ok(serde_json::from_value(result)?)
	}

	pub async fn get_block_count(&self) -> Result<u64> {
		let result = self.request("getblockcount", &[]).await?;
		Ok(serde_json::from_value(result)?)
	}

	pub async fn get_block_hash(&self, block_height: u64) -> Result<BlockHash> {
		let result = self.request("getblockhash", &[JsonValue::from(block_height)]).await?;
		Ok(serde_json::from_value(result)?)
	}

	pub async fn get_block(&self, hash: &BlockHash) -> Result<Block> {
		let result =
			self.request("getblock", &[JsonValue::from(hash.to_string()), 0.into()]).await?;
		deserialize_hex(result.as_str().unwrap())
	}

	async fn request(&self, method: &str, params: &[JsonValue]) -> Result<JsonValue> {
		let client = reqwest::Client::new();
		let mut req = client.post(&self.url);

		if let Auth::UserPass(username, password) = &self.auth {
			let token = general_purpose::STANDARD.encode(format!("{username}:{password}"));
			req = req.header(AUTHORIZATION, format!("Basic {token}"));
		}

		let retry_attempts = if self.with_retry { RETRY_ATTEMPTS } else { 1 };

		for attempt in 0..retry_attempts {
			let id = self.id.fetch_add(1, Ordering::Relaxed).to_string();
			let timeout = Duration::from_millis(RPC_TIMEOUT * 2_i32.pow(attempt) as u64);

			let body = json!({
				"jsonrpc": "2.0",
				"method": method,
				"params": params,
				"id": id,
			});

			match req.try_clone().unwrap().json(&body).send().await {
				Ok(response) => {
					let json = response.json::<Response>().await?;
					match json.error {
						Some(error) if error.code == RPC_IN_WARMUP => {
							sleep(timeout).await;
							continue;
						}
						Some(error) => {
							return Err(ClientError::Rpc { message: error.message }.into())
						}
						None if json.id.is_none() || json.id.unwrap() != id => {
							return Err(ClientError::NonceMismatch.into())
						}
						None => return Ok(json.result),
					}
				}
				Err(e) if e.is_connect() => {
					sleep(timeout).await;
					continue;
				}
				Err(e) => return Err(ClientError::General { message: e.to_string() }.into()),
			}
		}

		Err(ClientError::Connection.into())
	}
}

fn deserialize_hex<T: Decodable>(hex: &str) -> Result<T> {
	let mut reader = HexToBytesIter::new(hex)?;
	let object = Decodable::consensus_decode(&mut reader)?;

	if reader.read_u8().is_ok() {
		Err(eyre!("could not deserialize output"))
	} else {
		Ok(object)
	}
}
