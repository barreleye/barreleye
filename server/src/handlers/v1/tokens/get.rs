use axum::{
	extract::{Path, State},
	Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{BasicModel, Network, Token},
	utils, App,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	token: Token,
	networks: Vec<Network>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(token_id): Path<String>,
) -> ServerResult<'static, Json<Response>> {
	if let Some(token) = Token::get_by_id(app.db(), &token_id).await? {
		let networks =
			Network::get_all_by_network_ids(app.db(), token.network_id.into(), Some(false))
				.await?
				.into_iter()
				.map(|mut n| {
					n.rpc_endpoint = utils::with_masked_auth(&n.rpc_endpoint);
					n
				})
				.collect::<Vec<Network>>();

		Ok(Response { token, networks }.into())
	} else {
		Err(ServerError::NotFound)
	}
}
