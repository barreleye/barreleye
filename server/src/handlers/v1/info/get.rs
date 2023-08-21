use axum::{extract::State, Json};
use axum_extra::extract::Query;
use eyre::Result;
use sea_orm::ColumnTrait;
use serde::{Deserialize, Serialize};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{
		Address, Amount, Balance, BasicModel, Entity, EntityColumn, Link, Network, PrimaryId,
		PrimaryIds, SanitizedEntity, SanitizedNetwork, SanitizedTag, Tag,
	},
	App, RiskLevel,
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
pub struct ResponseSource {
	network: String,
	entity: String,
	address: String,
	hops: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	risk_level: RiskLevel,
	assets: Vec<ResponseAsset>,
	sources: Vec<ResponseSource>,
	networks: Vec<SanitizedNetwork>,
	entities: Vec<SanitizedEntity>,
	tags: Vec<SanitizedTag>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Query(payload): Query<Payload>,
) -> ServerResult<Json<Response>> {
	let addresses = {
		let mut ret = HashSet::new();

		let addresses = HashSet::<String>::from_iter(payload.addresses.iter().cloned());
		let entities = HashSet::<String>::from_iter(payload.entities.iter().cloned());
		let max_limit = 100;

		if addresses.len() > max_limit {
			return Err(ServerError::ExceededLimit {
				field: "address".to_string(),
				limit: max_limit,
			});
		}
		if entities.len() > max_limit {
			return Err(ServerError::ExceededLimit {
				field: "entity".to_string(),
				limit: max_limit,
			});
		}

		for address in addresses.into_iter() {
			if !address.is_empty() {
				let formatted_address = app.format_address(&address).await?;
				ret.insert(formatted_address);
			}
		}

		if !entities.is_empty() {
			let entity_ids: PrimaryIds = Entity::get_all_where(
				app.db(),
				EntityColumn::Id.is_in(entities.into_iter().collect::<Vec<String>>()),
			)
			.await?
			.into();

			if !entity_ids.is_empty() {
				for address in
					Address::get_all_by_entity_ids(app.db(), entity_ids, Some(false)).await?
				{
					ret.insert(address.address);
				}
			}
		}

		if ret.is_empty() {
			return Err(ServerError::MissingInputParams);
		}

		ret.into_iter().collect::<Vec<String>>()
	};

	// find links
	let links = Link::get_all_disinct_by_addresses(&app.warehouse, addresses.clone()).await?;

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

	async fn get_entities_data(
		app: Arc<App>,
		addresses: Vec<String>,
	) -> Result<(
		HashMap<(PrimaryId, String), PrimaryId>,
		HashMap<PrimaryId, Entity>,
		Vec<Tag>,
		RiskLevel,
	)> {
		let mut address_map = HashMap::new();
		let mut entities = HashMap::new();
		let mut tags = vec![];
		let mut risk_level = RiskLevel::Low;

		let addresses = Address::get_all_by_addresses(app.db(), addresses, Some(false)).await?;

		if !addresses.is_empty() {
			address_map = addresses
				.iter()
				.map(|a| ((a.network_id, a.address.clone()), a.entity_id))
				.collect::<HashMap<(PrimaryId, String), PrimaryId>>();

			let entity_ids = addresses.into_iter().map(|a| a.entity_id).collect::<Vec<PrimaryId>>();
			for entity in Entity::get_all_by_entity_ids(app.db(), entity_ids.into(), Some(false))
				.await?
				.into_iter()
			{
				entities.insert(entity.entity_id, entity);
			}

			if !entities.is_empty() {
				let joined_tags = Tag::get_all_by_entity_ids(
					app.db(),
					entities.clone().into_keys().collect::<Vec<PrimaryId>>().into(),
				)
				.await?;

				let mut map = HashMap::<PrimaryId, Vec<String>>::new();
				for joined_tag in joined_tags.iter() {
					if let Some(ids) = map.get_mut(&joined_tag.entity_id) {
						ids.push(joined_tag.id.clone());
					} else {
						map.insert(joined_tag.entity_id, vec![joined_tag.id.clone()]);
					}

					if joined_tag.risk_level > risk_level {
						risk_level = joined_tag.risk_level;
					}
				}

				for (entity_id, entity) in entities.iter_mut() {
					entity.tags = map.get(entity_id).cloned().or(Some(vec![]));
				}

				tags = joined_tags.into_iter().map(|jt| jt.into()).collect();
			}
		}

		Ok((address_map, entities, tags, risk_level))
	}

	pub async fn get_networks(app: Arc<App>, addresses: Vec<String>) -> Result<Vec<Network>> {
		let mut ret = vec![];

		let n = app.networks.read().await;
		let network_ids =
			Amount::get_all_network_ids_by_addresses(&app.warehouse, addresses).await?;
		if !network_ids.is_empty() {
			for (_, chain) in n.iter().filter(|(network_id, _)| network_ids.contains(network_id)) {
				ret.push(chain.get_network());
			}
		}

		Ok(ret)
	}

	let (assets, networks, entities_data) = tokio::join!(
		get_assets(app.clone(), addresses.clone()),
		get_networks(app.clone(), addresses.clone()),
		get_entities_data(app.clone(), {
			let mut from_addresses =
				links.iter().map(|l| l.from_address.clone()).collect::<Vec<String>>();

			from_addresses.sort_unstable();
			from_addresses.dedup();

			from_addresses
		}),
	);

	let (address_map, entities_map, tags, risk_level) = entities_data?;

	// assemble sources
	let mut sources = vec![];
	let n = app.networks.read().await;
	for link in links.into_iter() {
		let network_id = link.network_id as PrimaryId;
		if let Some(chain) = n.get(&network_id) {
			let network = chain.get_network();

			if let Some(&entity_id) = address_map.get(&(network_id, link.from_address.clone())) {
				if let Some(entity) = entities_map.get(&entity_id) {
					sources.push(ResponseSource {
						network: network.id,
						address: link.from_address,
						entity: entity.id.clone(),
						hops: link.transfer_uuids.len() as u64,
					});
				}
			}
		}
	}

	Ok(Response {
		risk_level,
		assets: assets?,
		sources,
		networks: networks?.into_iter().map(|n| n.into()).collect(),
		entities: entities_map.into_values().map(|e| e.into()).collect(),
		tags: tags.into_iter().map(|t| t.into()).collect(),
	}
	.into())
}
