use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::{App, ServerResult};
use barreleye_common::models::{BasicModel, Config, ConfigKey, Network};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseNetwork {
	name: String,
	tail_index: u64,
	block_height: u64,
	sync: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	sync: f64,
	networks: Vec<ResponseNetwork>,
}

pub async fn handler(State(app): State<Arc<App>>) -> ServerResult<Json<Response>> {
	let mut networks = vec![];

	for network in Network::get_all(&app.db).await?.into_iter() {
		let nid = network.network_id;

		let block_height = Config::get::<u64>(&app.db, ConfigKey::BlockHeight(nid))
			.await?
			.map(|v| v.value)
			.unwrap_or(0);

		let tail_index = Config::get::<u64>(&app.db, ConfigKey::IndexerTailSync(nid))
			.await?
			.map(|v| v.value)
			.unwrap_or(0);

		let sync = Config::get::<f64>(&app.db, ConfigKey::IndexerProgress(nid))
			.await?
			.map(|v| v.value)
			.unwrap_or(0.0);

		networks.push(ResponseNetwork { name: network.name, tail_index, block_height, sync });
	}

	Ok(Response {
		sync: networks.iter().map(|n| n.sync).sum::<f64>() / networks.len() as f64,
		networks,
	}
	.into())
}
