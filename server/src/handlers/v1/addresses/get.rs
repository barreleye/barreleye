use axum::{
	extract::{Path, State},
	Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{Address, Network, SoftDeleteModel},
	App,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	address: Address,
	networks: Vec<Network>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(address_id): Path<String>,
) -> ServerResult<Json<Response>> {
	if let Some(address) = Address::get_existing_by_id(app.db(), &address_id).await? {
		let networks =
			Network::get_all_by_network_ids(app.db(), address.network_id.into(), Some(false))
				.await?;
		Ok(Response { address, networks }.into())
	} else {
		Err(ServerError::NotFound)
	}
}
