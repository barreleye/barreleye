use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{BasicModel, Tag},
	App,
};

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(tag_id): Path<String>,
) -> ServerResult<StatusCode> {
	if Tag::delete_by_id(app.db(), &tag_id).await? {
		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(ServerError::NotFound)
	}
}
