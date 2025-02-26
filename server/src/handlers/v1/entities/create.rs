use axum::{extract::State, Json};
use sea_orm::{prelude::Json as JsonData, ColumnTrait};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, utils::extract_primary_ids, ServerResult};
use barreleye_common::{
	models::{is_valid_id, BasicModel, Entity, EntityTag, Tag, TagColumn},
	App, IdPrefix,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	id: Option<String>,
	name: Option<String>,
	description: String,
	data: Option<JsonData>,
	tags: Option<Vec<String>>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<'static, Json<Entity>> {
	// check that id is valid
	if let Some(id) = payload.id.clone() {
		if !is_valid_id(&id, IdPrefix::Entity) || Entity::get_by_id(app.db(), &id).await?.is_some()
		{
			return Err(ServerError::InvalidParam { field: "id".into(), value: id.into() });
		}
	}

	// check name
	if let Some(name) = payload.name.clone() {
		// check for soft-deleted matches
		if Entity::get_by_name(app.db(), &name, Some(true)).await?.is_some() {
			return Err(ServerError::TooEarly {
				reason: format!("entity hasn't been deleted yet: {name}").into(),
			});
		}

		// check for any duplicate
		if Entity::get_by_name(app.db(), &name, None).await?.is_some() {
			return Err(ServerError::Duplicate { field: "name".into(), value: name.into() });
		}
	}

	// check for invalid tags
	let mut tag_ids = vec![];
	if let Some(tags) = payload.tags {
		tag_ids = extract_primary_ids(
			"tags",
			tags.clone(),
			IdPrefix::Tag,
			Tag::get_all_where(app.db(), TagColumn::Id.is_in(tags.clone()))
				.await?
				.into_iter()
				.map(|t| (t.id, t.tag_id))
				.collect(),
		)?;
		if tag_ids.len() != tags.len() {
			return Err(ServerError::InvalidValues {
				field: "tags".into(),
				values: tags.join(", ").into(),
			});
		}
	}

	// create new
	let entity_id = Entity::create(
		app.db(),
		Entity::new_model(payload.id, payload.name, &payload.description, payload.data),
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
