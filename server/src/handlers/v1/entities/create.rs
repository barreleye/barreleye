use axum::{extract::State, Json};
use sea_orm::ColumnTrait;
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, utils::extract_primary_ids, ServerResult};
use barreleye_common::{
	models::{BasicModel, Entity, EntityTag, Tag, TagColumn},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	name: Option<String>,
	description: String,
	url: String,
	tags: Option<Vec<String>>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<Json<Entity>> {
	// get a list of tag primary ids, while checking for invalid payload ids
	let tag_ids = {
		let mut ret = vec![];

		if let Some(tags) = payload.tags {
			ret = extract_primary_ids(
				"tags",
				tags.clone(),
				Tag::get_all_where(app.db(), TagColumn::Id.is_in(tags))
					.await?
					.into_iter()
					.map(|t| (t.id, t.tag_id))
					.collect(),
			)?;
		}

		ret
	};

	// check for duplicate name
	if let Some(name) = payload.name.clone() {
		if Entity::get_by_name(app.db(), &name, None).await?.is_some() {
			return Err(ServerError::Duplicate { field: "name".to_string(), value: name });
		}
	}

	// create new
	let entity_id = Entity::create(
		app.db(),
		Entity::new_model(payload.name, &payload.description, &payload.url),
	)
	.await?;

	// upsert entity/tag mappings
	if !tag_ids.is_empty() {
		EntityTag::create_many(
			app.db(),
			tag_ids.into_iter().map(|tag_id| EntityTag::new_model(entity_id, tag_id)).collect(),
		)
		.await?;
	}

	// return newly created
	Ok(Entity::get(app.db(), entity_id).await?.unwrap().into())
}
