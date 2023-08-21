use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{is_valid_id, ApiKey, BasicModel},
	App, IdPrefix,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	id: Option<String>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<Json<ApiKey>> {
	// check that id is valid
	if let Some(id) = payload.id.clone() {
		if !is_valid_id(&id, IdPrefix::ApiKey) || ApiKey::get_by_id(app.db(), &id).await?.is_some()
		{
			return Err(ServerError::InvalidParam { field: "id".to_string(), value: id });
		}
	}

	// create new
	let api_key_id = ApiKey::create(app.db(), ApiKey::new_model(payload.id)).await?;

	// return newly created
	Ok(ApiKey::get(app.db(), api_key_id).await?.unwrap().format().into())
}
