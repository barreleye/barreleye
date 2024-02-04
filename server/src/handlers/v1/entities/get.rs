use axum::{
	extract::{Path, State},
	Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{
	errors::ServerError,
	handlers::v1::entities::{get_addresses_data, get_tags_data},
	ServerResult,
};
use barreleye_common::{
	models::{Address, Entity, Network, SoftDeleteModel, Tag},
	App,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	entity: Entity,
	tags: Vec<Tag>,
	addresses: Vec<Address>,
	networks: Vec<Network>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(entity_id): Path<String>,
) -> ServerResult<Json<Response>> {
	if let Some(mut entity) =
		Entity::get_existing_by_id(app.db(), &entity_id).await?
	{
		let (tags_data, addresses_data) = tokio::join!(
			get_tags_data(app.clone(), entity.entity_id.into()),
			get_addresses_data(app.clone(), entity.entity_id.into()),
		);

		let (tags, tags_map) = tags_data?;
		let (addresses, addresses_map, networks) = addresses_data?;

		entity.tags = tags_map.get(&entity.entity_id).cloned().or(Some(vec![]));
		entity.addresses =
			addresses_map.get(&entity.entity_id).cloned().or(Some(vec![]));

		Ok(Response { entity, tags, addresses, networks }.into())
	} else {
		Err(ServerError::NotFound)
	}
}
