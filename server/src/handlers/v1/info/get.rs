use axum::{extract::State, Json};
use axum_extra::extract::Query;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::{
	utils::{get_addresses_from_params, get_networks},
	ServerResult,
};
use barreleye_common::{
	models::{
		Address, Balance, Entity, PrimaryId, SanitizedEntity, SanitizedNetwork, SanitizedTag, Tag,
	},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	#[serde(default, rename = "address")]
	addresses: Vec<String>,
	#[serde(default, rename = "entity")]
	entities: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseAsset {
	network: String,
	address: Option<String>,
	balance: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	assets: Vec<ResponseAsset>,
	networks: Vec<SanitizedNetwork>,
	entities: Vec<SanitizedEntity>,
	tags: Vec<SanitizedTag>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Query(payload): Query<Payload>,
) -> ServerResult<Json<Response>> {
	// get addresses
	let addresses =
		get_addresses_from_params(app.clone(), payload.addresses, payload.entities).await?;

	// get assets
	async fn get_assets(app: Arc<App>, addresses: Vec<String>) -> Result<Vec<ResponseAsset>> {
		let mut ret = vec![];

		let n = app.networks.read().await;
		let all_balances = Balance::get_all_by_addresses(&app.warehouse, addresses).await?;
		if !all_balances.is_empty() {
			for balance_data in all_balances.into_iter() {
				if balance_data.balance.is_zero() {
					continue;
				}

				let network_id = balance_data.network_id as PrimaryId;
				if let Some(chain) = n.get(&network_id) {
					ret.push(ResponseAsset {
						network: chain.get_network().id,
						address: if balance_data.asset_address.is_empty() {
							None
						} else {
							Some(chain.format_address(&balance_data.asset_address))
						},
						balance: balance_data.balance.to_string(),
					});
				}
			}
		}

		Ok(ret)
	}

	// get entities
	async fn get_entities_data(
		app: Arc<App>,
		addresses: Vec<String>,
	) -> Result<(Vec<Entity>, Vec<Tag>)> {
		let mut entities = vec![];
		let mut tags = vec![];

		let addresses = Address::get_all_by_addresses(app.db(), addresses, Some(false)).await?;
		if !addresses.is_empty() {
			let entity_ids = addresses.into_iter().map(|a| a.entity_id).collect::<Vec<PrimaryId>>();
			entities =
				Entity::get_all_by_entity_ids(app.db(), entity_ids.into(), Some(false)).await?;

			if !entities.is_empty() {
				let joined_tags =
					Tag::get_all_by_entity_ids(app.db(), entities.clone().into()).await?;

				let mut map = HashMap::<PrimaryId, Vec<String>>::new();
				for joined_tag in joined_tags.iter() {
					if let Some(ids) = map.get_mut(&joined_tag.entity_id) {
						ids.push(joined_tag.id.clone());
					} else {
						map.insert(joined_tag.entity_id, vec![joined_tag.id.clone()]);
					}
				}

				for entity in entities.iter_mut() {
					entity.tags = map.get(&entity.entity_id).cloned().or(Some(vec![]));
				}

				tags = joined_tags.into_iter().map(|jt| jt.into()).collect();
			}
		}

		Ok((entities, tags))
	}

	let (assets, networks, entities_data) = tokio::join!(
		get_assets(app.clone(), addresses.clone()),
		get_networks(app.clone(), addresses.clone()),
		get_entities_data(app.clone(), addresses.clone()),
	);

	let (entities, tags) = entities_data?;

	Ok(Response {
		assets: assets?,
		networks: networks?.into_iter().map(|n| n.into()).collect(),
		entities: entities.into_iter().map(|e| e.into()).collect(),
		tags: tags.into_iter().map(|t| t.into()).collect(),
	}
	.into())
}
