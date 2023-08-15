use axum::{extract::State, Json};
use axum_extra::extract::Query;
use sea_orm::ColumnTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ServerResult;
use barreleye_common::{
	models::{BasicModel, Network, NetworkColumn},
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
	networks: Vec<Network>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Query(payload): Query<Payload>,
) -> ServerResult<Json<Response>> {
	let networks = Network::get_all_paginated_where(
		app.db(),
		NetworkColumn::IsDeleted.eq(false),
		payload.offset,
		payload.limit,
	)
	.await?;

	Ok(Response { networks }.into())
}
