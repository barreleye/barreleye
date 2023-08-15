use axum::{extract::State, Json};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ServerResult;
use barreleye_common::{
	models::{ApiKey, BasicModel},
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
	keys: Vec<ApiKey>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Query(payload): Query<Payload>,
) -> ServerResult<Json<Response>> {
	let keys = ApiKey::get_all_paginated(app.db(), payload.offset, payload.limit)
		.await?
		.iter()
		.map(|k| k.format())
		.collect::<Vec<ApiKey>>();

	Ok(Response { keys }.into())
}
