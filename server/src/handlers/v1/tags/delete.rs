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
	if let Some(tag) = Tag::get_by_id(app.db(), &tag_id).await? {
		// if locked (@TODO and "sanctions" mode is on), don't allow deleting
		if tag.is_locked {
			return Err(ServerError::BadRequest { reason: "object is locked".to_string() });
		}

		// delete
		if Tag::delete_by_id(app.db(), &tag_id).await? {
			return Ok(StatusCode::NO_CONTENT);
		}
	}

	Err(ServerError::NotFound)
}
