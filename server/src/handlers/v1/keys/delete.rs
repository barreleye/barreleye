use axum::{extract::State, http::StatusCode, Json};
use sea_orm::ColumnTrait;
use serde::Deserialize;
use std::{collections::HashSet, sync::Arc};

use crate::ServerResult;
use barreleye_common::{
	models::{ApiKey, ApiKeyColumn, BasicModel},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	keys: HashSet<String>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<StatusCode> {
	// exit if no input
	if payload.keys.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// delete all keys
	ApiKey::delete_all_where(
		app.db(),
		ApiKeyColumn::Id
			.is_in(payload.keys.into_iter().collect::<Vec<String>>()),
	)
	.await?;

	Ok(StatusCode::NO_CONTENT)
}
