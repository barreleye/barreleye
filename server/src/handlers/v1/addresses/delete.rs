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
	if let Some(address) = Address::get_existing_by_id(app.db(), &address_id).await? {
		// if locked (@TODO and "sanctions" mode is on), don't allow deleting
		if address.is_locked {
			return Err(ServerError::BadRequest { reason: "object is locked".to_string() });
		}

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
