use axum::{extract::State, Json};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ServerResult;
use barreleye_common::{
	models::{BasicModel, Network, PrimaryId, Token},
	utils, App,
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
	tokens: Vec<Token>,
	networks: Vec<Network>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Query(payload): Query<Payload>,
) -> ServerResult<Json<Response>> {
	let tokens = Token::get_all_paginated(app.db(), payload.offset, payload.limit).await?;

	let network_ids = tokens.iter().map(|t| t.network_id).collect::<Vec<PrimaryId>>();
	let networks = Network::get_all_by_network_ids(app.db(), network_ids.into(), Some(false))
		.await?
		.into_iter()
		.map(|mut n| {
			n.rpc_endpoint = utils::with_masked_auth(&n.rpc_endpoint);
			n
		})
		.collect::<Vec<Network>>();

	Ok(Response { tokens, networks }.into())
}
