use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use sea_orm::{prelude::Json as JsonData, ActiveModelTrait, ColumnTrait};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, utils::extract_primary_ids, ServerResult};
use barreleye_common::{
	models::{
		optional_set, BasicModel, Entity, EntityActiveModel, EntityTag, SoftDeleteModel, Tag,
		TagColumn,
	},
	App, IdPrefix,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	name: Option<Option<String>>,
	description: Option<String>,
	data: Option<JsonData>,
	tags: Option<Vec<String>>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(entity_id): Path<String>,
	Json(payload): Json<Payload>,
) -> ServerResult<'static, StatusCode> {
	if let Some(entity) = Entity::get_existing_by_id(app.db(), &entity_id).await? {
		// check name
		if let Some(Some(name)) = payload.name.clone() {
			// check for soft-deleted matches
			if Entity::get_by_name(app.db(), &name, Some(true)).await?.is_some() {
				return Err(ServerError::TooEarly {
					reason: format!("entity hasn't been deleted yet: {name}").into(),
				});
			}

			// check for any duplicate
			if let Some(other_entity) = Entity::get_by_name(app.db(), &name, None).await? {
				if other_entity.id != entity.id {
					return Err(ServerError::Duplicate {
						field: "name".into(),
						value: name.into(),
					});
				}
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

		// update entity
		let update_data = EntityActiveModel {
			name: optional_set(payload.name),
			description: optional_set(payload.description),
			data: optional_set(payload.data),
			..Default::default()
		};
		if update_data.is_changed() {
			Entity::update_by_id(app.db(), &entity_id, update_data).await?;
		}

		// upsert entity/tag mappings
		if !tag_ids.is_empty() {
			EntityTag::delete_not_included_tags(app.db(), entity.entity_id, tag_ids.clone().into())
				.await?;
			EntityTag::create_many(
				app.db(),
				tag_ids
					.iter()
					.map(|tag_id| EntityTag::new_model(entity.entity_id, *tag_id))
					.collect(),
			)
			.await?;
		}

		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(ServerError::NotFound)
	}
}
