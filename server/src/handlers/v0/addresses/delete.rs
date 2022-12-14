use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use std::sync::Arc;

use crate::{errors::ServerError, App, ServerResult};
use barreleye_common::models::{BasicModel, LabeledAddress};

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(label_address_id): Path<String>,
) -> ServerResult<StatusCode> {
	if LabeledAddress::delete_by_id(&app.db, &label_address_id).await? {
		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(ServerError::NotFound)
	}
}
