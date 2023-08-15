use axum::{
	routing::{delete, get, post, put},
	Router,
};
use eyre::Result;
use std::{collections::HashMap, sync::Arc};

use barreleye_common::{
	models::{Address, Entity, Network, PrimaryId, PrimaryIds},
	App,
};

mod create;
mod delete;
mod get;
mod list;
mod update;

pub fn get_routes() -> Router<Arc<App>> {
	Router::new()
		.route("/", post(create::handler))
		.route("/", get(list::handler))
		.route("/:id", get(get::handler))
		.route("/:id", put(update::handler))
		.route("/:id", delete(delete::handler))
}

pub async fn get_data_by_tag_ids(
	app: Arc<App>,
	tag_ids: PrimaryIds,
) -> Result<(HashMap<PrimaryId, Vec<String>>, Vec<Entity>, Vec<Address>, Vec<Network>)> {
	let joined_entities = Entity::get_all_by_tag_ids(app.db(), tag_ids, Some(false)).await?;

	let addresses =
		Address::get_all_by_entity_ids(app.db(), joined_entities.clone().into(), Some(false))
			.await?;

	let network_ids = addresses.iter().map(|a| a.network_id).collect::<Vec<PrimaryId>>();
	let networks =
		Network::get_all_by_network_ids(app.db(), network_ids.into(), Some(false)).await?;

	let mut tags_map = HashMap::<PrimaryId, Vec<String>>::new();
	for joined_entity in joined_entities.iter() {
		if let Some(ids) = tags_map.get_mut(&joined_entity.tag_id) {
			ids.push(joined_entity.id.clone());
		} else {
			tags_map.insert(joined_entity.tag_id, vec![joined_entity.id.clone()]);
		}
	}

	let mut entities_map = HashMap::<PrimaryId, Vec<String>>::new();
	for address in addresses.iter() {
		if let Some(ids) = entities_map.get_mut(&address.entity_id) {
			ids.push(address.id.clone());
		} else {
			entities_map.insert(address.entity_id, vec![address.id.clone()]);
		}
	}

	let entities = joined_entities
		.into_iter()
		.map(|je| {
			let mut entity: Entity = je.into();
			entity.addresses = entities_map.get(&entity.entity_id).cloned().or(Some(vec![]));
			entity
		})
		.collect();

	Ok((tags_map, entities, addresses, networks))
}
