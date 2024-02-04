use axum::{
	extract::{Path, State},
	Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{
	errors::ServerError, handlers::v1::tags::get_data_by_tag_ids, ServerResult,
};
use barreleye_common::{
	models::{Address, BasicModel, Entity, Network, Tag},
	App,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	tag: Tag,
	entities: Vec<Entity>,
	addresses: Vec<Address>,
	networks: Vec<Network>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(tag_id): Path<String>,
) -> ServerResult<Json<Response>> {
	if let Some(mut tag) = Tag::get_by_id(app.db(), &tag_id).await? {
		let (tags_map, entities, addresses, networks) =
			get_data_by_tag_ids(app.clone(), tag.tag_id.into()).await?;

		tag.entities = tags_map.get(&tag.tag_id).cloned().or(Some(vec![]));
		Ok(Response { tag, entities, addresses, networks }.into())
	} else {
		Err(ServerError::NotFound)
	}
}
