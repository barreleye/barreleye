use axum::{extract::State, Json};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{Address, BasicModel, Config, ConfigKey, Entity, Network, PrimaryId, SoftDeleteModel},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	entity: String,
	network: String,
	addresses: HashMap<String, String>, // address -> description
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<Json<Vec<Address>>> {
	let entity = Entity::get_existing_by_id(app.db(), &payload.entity)
		.await?
		.ok_or(ServerError::InvalidParam { field: "entity".to_string(), value: payload.entity })?;

	let network =
		Network::get_by_id(app.db(), &payload.network).await?.ok_or(ServerError::InvalidParam {
			field: "network".to_string(),
			value: payload.network,
		})?;

	// check for soft-deleted records
	let addresses = Address::get_all_by_network_id_and_addresses(
		app.db(),
		network.network_id,
		payload.addresses.clone().into_keys().collect(),
		Some(true),
	)
	.await?;
	if !addresses.is_empty() {
		return Err(ServerError::Conflict {
			reason: format!(
				"the following addresses have not been properly deleted yet: {}; try again later",
				addresses.into_iter().map(|a| a.address).collect::<Vec<String>>().join(", ")
			),
		});
	}

	// check for duplicates
	let addresses = Address::get_all_by_network_id_and_addresses(
		app.db(),
		network.network_id,
		payload.addresses.clone().into_keys().collect(),
		Some(false),
	)
	.await?;
	if !addresses.is_empty() {
		return Err(ServerError::Duplicates {
			field: "addresses".to_string(),
			values: addresses.into_iter().map(|a| a.address).collect::<Vec<String>>().join(", "),
		});
	}

	// create new
	Address::create_many(
		app.db(),
		payload
			.addresses
			.clone()
			.iter()
			.map(|(address, description)| {
				Address::new_model(
					entity.entity_id,
					network.network_id,
					&network.id,
					address,
					description,
				)
			})
			.collect(),
	)
	.await?;

	// tell upstream indexer about newly created addresses
	Config::set_many::<_, PrimaryId>(
		app.db(),
		Address::get_all_by_network_id_and_addresses(
			app.db(),
			network.network_id,
			payload.addresses.clone().into_keys().collect(),
			Some(false),
		)
		.await?
		.into_iter()
		.map(|a| (ConfigKey::NewlyAddedAddress(a.network_id, a.address_id), a.address_id))
		.collect::<HashMap<ConfigKey, PrimaryId>>(),
	)
	.await?;

	// return newly created
	Ok(Address::get_all_by_network_id_and_addresses(
		app.db(),
		network.network_id,
		payload.addresses.into_keys().collect(),
		Some(false),
	)
	.await?
	.into())
}
