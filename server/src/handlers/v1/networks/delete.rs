use axum::{extract::State, http::StatusCode, Json};
use sea_orm::ColumnTrait;
use serde::Deserialize;
use std::{collections::HashSet, sync::Arc};

use crate::ServerResult;
use barreleye_common::{
	models::{
		set, Address, AddressActiveModel, BasicModel, Config, ConfigKey,
		Network, NetworkActiveModel, NetworkColumn, PrimaryId,
	},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	networks: HashSet<String>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<StatusCode> {
	// exit if no input
	if payload.networks.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// get all networks
	let all_networks = Network::get_all_where(
		app.db(),
		NetworkColumn::Id.is_in(payload.networks),
	)
	.await?;

	// proceed only when there's something to delete
	if all_networks.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// soft-delete all associated addresses
	let all_network_ids =
		all_networks.iter().map(|n| n.network_id).collect::<Vec<PrimaryId>>();
	Address::update_all_where(
		app.db(),
		NetworkColumn::NetworkId.is_in(all_network_ids.clone()),
		AddressActiveModel { is_deleted: set(true), ..Default::default() },
	)
	.await?;

	// soft-delete networks
	Network::update_all_where(
		app.db(),
		NetworkColumn::NetworkId.is_in(all_network_ids),
		NetworkActiveModel { is_deleted: set(true), ..Default::default() },
	)
	.await?;

	// update config
	Config::set::<_, u8>(app.db(), ConfigKey::NetworksUpdated, 1).await?;

	// update app's networks
	let mut networks = app.networks.write().await;
	*networks = app.get_networks().await?;

	Ok(StatusCode::NO_CONTENT)
}
