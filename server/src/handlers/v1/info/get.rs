use axum::{extract::State, Json};
use axum_extra::extract::Query;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{
		Address, Amount, Balance, BasicModel, Entity, Link, Network, PrimaryId, SanitizedEntity,
		SanitizedNetwork, SanitizedTag, Tag,
	},
	App, RiskLevel, RiskReason,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	q: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseRisk {
	level: RiskLevel,
	reasons: HashSet<RiskReason>,
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
	from: String,
	to: String,
	hops: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	addresses: Vec<String>,
	risk: ResponseRisk,
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

		let q = payload.q.trim();
		if q.is_empty() {
			return Err(ServerError::MissingInputParams);
		}

		if let Some(entity) = Entity::get_by_id(app.db(), q).await? {
			for address in
				Address::get_all_by_entity_ids(app.db(), vec![entity.entity_id].into(), Some(false))
					.await?
			{
				ret.insert(address.address);
			}
		} else {
			ret.insert(q.to_string());
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
			let mut entity_addresses =
				links.iter().map(|l| l.from_address.clone()).collect::<HashSet<String>>();

			for address in addresses.clone() {
				entity_addresses.insert(address);
			}

			entity_addresses.into_iter().collect::<Vec<_>>()
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
						from: link.from_address,
						to: link.to_address,
						entity: entity.id.clone(),
						hops: link.transfer_uuids.len() as u64,
					});
				}
			}
		}
	}

	let mut risk_reasons = HashSet::new();
	for (_, network_address) in address_map.keys() {
		if addresses.contains(network_address) {
			risk_reasons.insert(RiskReason::Entity);
			break;
		}
	}
	if !sources.is_empty() {
		risk_reasons.insert(RiskReason::Source);
	}

	Ok(Response {
		addresses,
		risk: ResponseRisk { level: risk_level, reasons: risk_reasons },
		assets: assets?,
		sources,
		networks: networks?.into_iter().map(|n| n.into()).collect(),
		entities: entities_map.into_values().map(|e| e.into()).collect(),
		tags: tags.into_iter().map(|t| t.into()).collect(),
	}
	.into())
}
