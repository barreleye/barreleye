use axum::{extract::State, http::StatusCode, Json};
use sea_orm::ColumnTrait;
use serde::Deserialize;
use std::{collections::HashSet, sync::Arc};

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{BasicModel, PrimaryId, Tag, TagColumn},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	tags: HashSet<String>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<StatusCode> {
	// exit if no input
	if payload.tags.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// get all tags
	let all_tags = Tag::get_all_where(app.db(), TagColumn::Id.is_in(payload.tags)).await?;

	// proceed only when there's something to delete
	if all_tags.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// make sure none of the tags are locked if sanctions mode is active
	if !app.settings.sanction_lists.is_empty() {
		let invalid_tags = all_tags.iter().filter(|t| t.is_locked).collect::<Vec<&Tag>>();
		if !invalid_tags.is_empty() {
			return Err(ServerError::Locked);
		}
	}

	// soft-delete all associated tags
	Tag::delete_all_where(
		app.db(),
		TagColumn::TagId.is_in(all_tags.iter().map(|t| t.tag_id).collect::<Vec<PrimaryId>>()),
	)
	.await?;

	Ok(StatusCode::NO_CONTENT)
}
