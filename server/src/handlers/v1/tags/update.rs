use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use sea_orm::ActiveModelTrait;
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{optional_set, BasicModel, Tag, TagActiveModel},
	App, RiskLevel,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	name: Option<String>,
	risk_level: Option<RiskLevel>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(tag_id): Path<String>,
	Json(payload): Json<Payload>,
) -> ServerResult<StatusCode> {
	if let Some(tag) = Tag::get_by_id(app.db(), &tag_id).await? {
		// if sanctions mode is on, don't allow editing a locked tag
		if !app.settings.sanction_lists.is_empty() && tag.is_locked {
			return Err(ServerError::Locked);
		}

		// check for duplicate name
		if let Some(name) = payload.name.clone() {
			if let Some(other_tag) = Tag::get_by_name(app.db(), &name).await? {
				if other_tag.id != tag.id {
					return Err(ServerError::Duplicate { field: "name".to_string(), value: name });
				}
			}
		}

		// update
		let update_data = TagActiveModel {
			name: optional_set(payload.name),
			risk_level: optional_set(payload.risk_level),
			..Default::default()
		};
		if update_data.is_changed() {
			Tag::update_by_id(app.db(), &tag_id, update_data).await?;
		}

		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(ServerError::NotFound)
	}
}
