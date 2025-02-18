use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	chain::{Bitcoin, ChainTrait, Evm},
	models::{is_valid_id, BasicModel, Config, ConfigKey, Network},
	App, Architecture, IdPrefix,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	id: Option<String>,
	name: String,
	architecture: Architecture,
	block_time: u64,
	rpc_endpoint: String,
	chain_id: Option<u64>,
	rps: Option<u32>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<'static, Json<Network>> {
	let chain_id = payload.chain_id.unwrap_or_default();
	let rps = payload.rps.unwrap_or(100);

	// check that id is valid
	if let Some(id) = payload.id.clone() {
		if !is_valid_id(&id, IdPrefix::Network) ||
			Network::get_by_id(app.db(), &id).await?.is_some()
		{
			return Err(ServerError::InvalidParam { field: "id".into(), value: id.into() });
		}
	}

	// check name for soft-deleted matches
	if Network::get_by_name(app.db(), &payload.name, Some(true)).await?.is_some() {
		return Err(ServerError::TooEarly {
			reason: format!("network hasn't been deleted yet: {}", payload.name).into(),
		});
	}

	// check name for any duplicate
	if Network::get_by_name(app.db(), &payload.name, None).await?.is_some() {
		return Err(ServerError::Duplicate { field: "name".into(), value: payload.name.into() });
	}

	// check for duplicate chain id
	if Network::get_by_architecture_and_chain_id(
		app.db(),
		payload.architecture,
		chain_id as i64,
		None,
	)
	.await?
	.is_some()
	{
		return Err(ServerError::Duplicate {
			field: "chainId".into(),
			value: chain_id.to_string().into(),
		});
	}

	// check rpc connection
	let n = Network { rpc_endpoint: payload.rpc_endpoint.clone(), ..Default::default() };
	let mut boxed_chain: Box<dyn ChainTrait> = match payload.architecture {
		Architecture::Bitcoin => Box::new(Bitcoin::new(n)),
		Architecture::Evm => Box::new(Evm::new(n)),
	};
	if !boxed_chain.connect().await? {
		return Err(ServerError::InvalidService { name: boxed_chain.get_network().name.into() });
	}

	// create new
	let network_id = Network::create(
		app.db(),
		Network::new_model(
			payload.id,
			&payload.name,
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
