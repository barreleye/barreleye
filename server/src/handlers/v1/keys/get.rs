use axum::{
	extract::{Path, State},
	Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{ApiKey, BasicModel},
	App,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	key: ApiKey,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(api_key_id): Path<String>,
) -> ServerResult<Json<Response>> {
	ApiKey::get_by_id(app.db(), &api_key_id)
		.await?
		.map(|key| Response { key: key.format() }.into())
		.ok_or(ServerError::NotFound)
}
