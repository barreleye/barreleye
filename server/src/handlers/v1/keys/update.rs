use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use sea_orm::ActiveModelTrait;
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{optional_set, ApiKey, ApiKeyActiveModel, BasicModel},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	is_active: Option<bool>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(api_key_id): Path<String>,
	Json(payload): Json<Payload>,
) -> ServerResult<StatusCode> {
	match ApiKey::get_by_id(app.db(), &api_key_id).await? {
		Some(_) => {
			let update_data = ApiKeyActiveModel {
				is_active: optional_set(payload.is_active),
				..Default::default()
			};
			if update_data.is_changed() {
				ApiKey::update_by_id(app.db(), &api_key_id, update_data).await?;
			}

			Ok(StatusCode::NO_CONTENT)
		}
		_ => Err(ServerError::NotFound),
	}
}
