use axum::{extract::State, Json};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{handlers::v1::tags::get_data_by_tag_ids, ServerResult};
use barreleye_common::{
	models::{Address, BasicModel, Entity, Network, Tag},
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
	tags: Vec<Tag>,
	entities: Vec<Entity>,
	addresses: Vec<Address>,
	networks: Vec<Network>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Query(payload): Query<Payload>,
) -> ServerResult<'static, Json<Response>> {
	let mut tags = Tag::get_all_paginated(app.db(), payload.offset, payload.limit).await?;

	let (tags_map, entities, addresses, networks) =
		get_data_by_tag_ids(app.clone(), tags.clone().into()).await?;

	for tag in tags.iter_mut() {
		tag.entities = tags_map.get(&tag.tag_id).cloned().or(Some(vec![]));
	}

	Ok(Response { tags, entities, addresses, networks }.into())
}
