use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::ServerResult;
use barreleye_common::{
	models::{BasicModel, Config, ConfigKey, Network},
	App,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseNetwork {
	name: String,
	block_height: u64,
	synced: f64,
	processed: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	networks: Vec<ResponseNetwork>,
}

pub async fn handler(State(app): State<Arc<App>>) -> ServerResult<Json<Response>> {
	let mut networks = vec![];

	for network in Network::get_all(app.db()).await?.into_iter() {
		let nid = network.network_id;

		let block_height = Config::get::<_, u64>(app.db(), ConfigKey::BlockHeight(nid))
			.await?
			.map(|v| v.value)
			.unwrap_or(0);

		let synced = Config::get::<_, f64>(app.db(), ConfigKey::IndexerSyncProgress(nid))
			.await?
			.map(|v| v.value)
			.unwrap_or(0.0);

		let processed = Config::get::<_, f64>(app.db(), ConfigKey::IndexerProcessProgress(nid))
			.await?
			.map(|v| v.value)
			.unwrap_or(0.0);

		networks.push(ResponseNetwork {
			name: network.name,
			block_height,
			synced: (synced * 1000000.0).round() / 1000000.0,
			processed: (processed * 1000000.0).round() / 1000000.0,
		});
	}

	Ok(Response { networks }.into())
}
