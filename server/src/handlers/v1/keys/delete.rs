use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{ApiKey, BasicModel},
	App,
};

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(api_key_id): Path<String>,
) -> ServerResult<StatusCode> {
	if ApiKey::delete_by_id(app.db(), &api_key_id).await? {
		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(ServerError::NotFound)
	}
}
