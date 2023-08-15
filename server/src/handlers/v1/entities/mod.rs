use axum::{
	routing::{delete, get, post, put},
	Router,
};
use eyre::Result;
use std::{collections::HashMap, sync::Arc};

use barreleye_common::{
	models::{Address, Network, PrimaryId, PrimaryIds, Tag},
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

pub async fn get_tags_data(
	app: Arc<App>,
	entity_ids: PrimaryIds,
) -> Result<(Vec<Tag>, HashMap<PrimaryId, Vec<String>>)> {
	let joined_tags = Tag::get_all_by_entity_ids(app.db(), entity_ids).await?;
	let mut map = HashMap::<PrimaryId, Vec<String>>::new();

	for joined_tag in joined_tags.iter() {
		if let Some(ids) = map.get_mut(&joined_tag.entity_id) {
			ids.push(joined_tag.id.clone());
		} else {
			map.insert(joined_tag.entity_id, vec![joined_tag.id.clone()]);
		}
	}

	Ok((joined_tags.into_iter().map(|jt| jt.into()).collect::<Vec<Tag>>(), map))
}

pub async fn get_addresses_data(
	app: Arc<App>,
	entity_ids: PrimaryIds,
) -> Result<(Vec<Address>, HashMap<PrimaryId, Vec<String>>, Vec<Network>)> {
	let addresses = Address::get_all_by_entity_ids(app.db(), entity_ids, Some(false)).await?;

	let network_ids = addresses.iter().map(|a| a.network_id).collect::<Vec<PrimaryId>>();
	let networks_map = Network::get_all_by_network_ids(app.db(), network_ids.into(), Some(false))
		.await?
		.into_iter()
		.map(|n| (n.network_id, n))
		.collect::<HashMap<PrimaryId, Network>>();

	let networks = networks_map.into_values().collect::<Vec<Network>>();

	let mut map = HashMap::<PrimaryId, Vec<String>>::new();
	for address in addresses.iter() {
		if let Some(ids) = map.get_mut(&address.entity_id) {
			ids.push(address.id.clone());
		} else {
			map.insert(address.entity_id, vec![address.id.clone()]);
		}
	}

	Ok((addresses, map, networks))
}
