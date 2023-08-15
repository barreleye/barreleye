use axum::{
	extract::{Path, State},
	Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{Network, SoftDeleteModel},
	App,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	network: Network,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(network_id): Path<String>,
) -> ServerResult<Json<Response>> {
	Network::get_existing_by_id(app.db(), &network_id)
		.await?
		.map(|network| Response { network }.into())
		.ok_or(ServerError::NotFound)
}
