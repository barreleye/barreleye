use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{set, Address, AddressActiveModel, BasicModel, SoftDeleteModel},
	App,
};

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(address_id): Path<String>,
) -> ServerResult<StatusCode> {
	if Address::get_existing_by_id(app.db(), &address_id).await?.is_some() {
		// soft-delete address
		Address::update_by_id(
			app.db(),
			&address_id,
			AddressActiveModel { is_deleted: set(true), ..Default::default() },
		)
		.await?;

		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(ServerError::NotFound)
	}
}
