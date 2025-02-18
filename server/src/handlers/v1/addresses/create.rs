use axum::{extract::State, Json};
use sea_orm::prelude::Json as JsonData;
use serde::Deserialize;
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{Address, BasicModel, Config, ConfigKey, Entity, Network, PrimaryId, SoftDeleteModel},
	App,
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadAddress {
	address: String,
	description: String,
	data: Option<JsonData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	entity: String,
	network: String,
	addresses: Vec<PayloadAddress>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<'static, Json<Vec<Address>>> {
	// fetch entity
	let entity = Entity::get_existing_by_id(app.db(), &payload.entity).await?.ok_or(
		ServerError::InvalidParam { field: "entity".into(), value: payload.entity.into() },
	)?;

	// ensure addresses are unique
	let unique_addresses: HashSet<String> =
		HashSet::from_iter(payload.addresses.iter().map(|a| a.address.clone()));
	if unique_addresses.len() < payload.addresses.len() {
		return Err(ServerError::BadRequest {
			reason: "request contains duplicate addresses".into(),
		});
	}

	// get network
	let network =
		Network::get_by_id(app.db(), &payload.network).await?.ok_or(ServerError::InvalidParam {
			field: "network".into(),
			value: payload.network.into(),
		})?;

	// check for soft-deleted address conflicts
	let addresses = Address::get_all_by_entity_id_network_id_and_addresses(
		app.db(),
		entity.entity_id,
		network.network_id,
		unique_addresses.clone().into_iter().collect::<Vec<String>>(),
		Some(true),
	)
	.await?;
	if !addresses.is_empty() {
		return Err(ServerError::TooEarly {
			reason: format!(
				"addresses haven't been deleted yet: {}",
				addresses.into_iter().map(|a| a.address).collect::<Vec<String>>().join(", ")
			)
			.into(),
		});
	}

	// check for duplicates
	let addresses = Address::get_all_by_entity_id_network_id_and_addresses(
		app.db(),
		entity.entity_id,
		network.network_id,
		unique_addresses.clone().into_iter().collect::<Vec<String>>(),
		Some(false),
	)
	.await?;
	if !addresses.is_empty() {
		return Err(ServerError::Duplicates {
			field: "addresses".into(),
			values: addresses
				.into_iter()
				.map(|a| a.address)
				.collect::<Vec<String>>()
				.join(", ")
				.into(),
		});
	}

	// create new
	Address::create_many(
		app.db(),
		payload
			.addresses
			.clone()
			.iter()
			.map(|address| {
				Address::new_model(
					None,
					entity.entity_id,
					network.network_id,
					&network.id,
					&address.address,
					&address.description,
					address.data.clone(),
				)
			})
			.collect(),
	)
	.await?;

	// tell upstream indexer about newly created addresses
	Config::set_many::<_, PrimaryId>(
		app.db(),
		Address::get_all_by_entity_id_network_id_and_addresses(
			app.db(),
			entity.entity_id,
			network.network_id,
			unique_addresses.clone().into_iter().collect::<Vec<String>>(),
			Some(false),
		)
		.await?
		.into_iter()
		.map(|a| (ConfigKey::NewlyAddedAddress(a.network_id, a.address_id), a.address_id))
		.collect::<HashMap<ConfigKey, PrimaryId>>(),
	)
	.await?;

	// return newly created
	Ok(Address::get_all_by_entity_id_network_id_and_addresses(
		app.db(),
		entity.entity_id,
		network.network_id,
		unique_addresses.clone().into_iter().collect::<Vec<String>>(),
		Some(false),
	)
	.await?
	.into())
}
