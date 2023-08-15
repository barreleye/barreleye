use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	chain::{Bitcoin, ChainTrait, Evm},
	models::{BasicModel, Config, ConfigKey, Network},
	App, Architecture, Env,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	name: String,
	architecture: Architecture,
	block_time: u64,
	rpc_endpoint: String,
	env: Option<Env>,
	chain_id: Option<u64>,
	rps: Option<u32>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<Json<Network>> {
	let env = payload.env.unwrap_or_default();
	let chain_id = payload.chain_id.unwrap_or_default();
	let rps = payload.rps.unwrap_or(100);

	// check for duplicate name
	if Network::get_by_name(app.db(), &payload.name, None).await?.is_some() {
		return Err(ServerError::Duplicate { field: "name".to_string(), value: payload.name });
	}

	// check for duplicate chain id
	if Network::get_by_env_architecture_and_chain_id(
		app.db(),
		env,
		payload.architecture,
		chain_id as i64,
		None,
	)
	.await?
	.is_some()
	{
		return Err(ServerError::Duplicate {
			field: "chainId".to_string(),
			value: chain_id.to_string(),
		});
	}

	// check rpc connection
	let c = app.cache.clone();
	let n = Network { rpc_endpoint: payload.rpc_endpoint.clone(), ..Default::default() };
	let mut boxed_chain: Box<dyn ChainTrait> = match payload.architecture {
		Architecture::Bitcoin => Box::new(Bitcoin::new(c, n)),
		Architecture::Evm => Box::new(Evm::new(c, n)),
	};
	if !boxed_chain.connect().await? {
		return Err(ServerError::InvalidService { name: boxed_chain.get_network().name });
	}

	// create new
	let network_id = Network::create(
		app.db(),
		Network::new_model(
			&payload.name,
			env,
			payload.architecture,
			chain_id as i64,
			payload.block_time as i64,
			payload.rpc_endpoint,
			rps as i32,
		),
	)
	.await?;

	// update config
	Config::set::<_, u8>(app.db(), ConfigKey::NetworksUpdated, 1).await?;

	// update app's networks
	let mut networks = app.networks.write().await;
	*networks = app.get_networks().await?;

	// return newly created
	Ok(Network::get(app.db(), network_id).await?.unwrap().into())
}
