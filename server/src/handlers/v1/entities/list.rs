use axum::{
	extract::{Query, State},
	Json,
};
use sea_orm::ColumnTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
	handlers::v1::entities::{get_addresses_data, get_tags_data},
	ServerResult,
};
use barreleye_common::{
	models::{Address, BasicModel, Entity, EntityColumn, Network, Tag},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	offset: Option<u64>,
	limit: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	entities: Vec<Entity>,
	tags: Vec<Tag>,
	addresses: Vec<Address>,
	networks: Vec<Network>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Query(payload): Query<Payload>,
) -> ServerResult<Json<Response>> {
	let mut entities = Entity::get_all_paginated_where(
		app.db(),
		EntityColumn::IsDeleted.eq(false),
		payload.offset,
		payload.limit,
	)
	.await?;

	let (tags_data, addresses_data) = tokio::join!(
		get_tags_data(app.clone(), entities.clone().into()),
		get_addresses_data(app.clone(), entities.clone().into()),
	);

	let (tags, tags_map) = tags_data?;
	let (addresses, addresses_map, networks) = addresses_data?;

	for entity in entities.iter_mut() {
		entity.tags = tags_map.get(&entity.entity_id).cloned().or(Some(vec![]));
		entity.addresses = addresses_map.get(&entity.entity_id).cloned().or(Some(vec![]));
	}

	Ok(Response { entities, tags, addresses, networks }.into())
}
