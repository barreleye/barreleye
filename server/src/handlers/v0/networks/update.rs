use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use sea_orm::ActiveModelTrait;
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{
		optional_set, BasicModel, Config, ConfigKey, Network, NetworkActiveModel, SoftDeleteModel,
	},
	App, Architecture, Env,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	name: Option<String>,
	env: Option<Env>,
	architecture: Option<Architecture>,
	chain_id: Option<u64>,
	block_time: Option<u64>,
	rpc_endpoint: Option<String>,
	rps: Option<u32>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(network_id): Path<String>,
	Json(payload): Json<Payload>,
) -> ServerResult<StatusCode> {
	let network =
		Network::get_existing_by_id(app.db(), &network_id).await?.ok_or(ServerError::NotFound)?;

	// check for duplicate name
	if let Some(name) = payload.name.clone() {
		if network_id != network.id &&
			network.name.trim().to_lowercase() == name.trim().to_lowercase()
		{
			return Err(ServerError::Duplicate { field: "name".to_string(), value: name });
		}
	}

	// check for duplicate chain id
	if let Some(chain_id) = payload.chain_id {
		if Network::get_by_env_architecture_and_chain_id(
			app.db(),
			payload.env.unwrap_or(network.env),
			payload.architecture.unwrap_or(network.architecture),
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
	}

	let update_data = NetworkActiveModel {
		name: optional_set(payload.name.clone()),
		env: optional_set(payload.env),
		architecture: optional_set(payload.architecture),
		chain_id: optional_set(payload.chain_id.map(|v| v as i64)),
		block_time: optional_set(payload.block_time.map(|v| v as i64)),
		rpc_endpoint: optional_set(payload.rpc_endpoint.clone()),
		rps: optional_set(payload.rps.map(|v| v as i32)),
		..Default::default()
	};

	if update_data.is_changed() {
		// update network
		Network::update_by_id(app.db(), &network_id, update_data).await?;

		// update config
		Config::set::<_, u8>(app.db(), ConfigKey::NetworksUpdated, 1).await?;

		// update app's networks
		let mut networks = app.networks.write().await;
		*networks = app.get_networks().await?;
	}

	Ok(StatusCode::NO_CONTENT)
}
