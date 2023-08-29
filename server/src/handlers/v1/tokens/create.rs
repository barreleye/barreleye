use axum::{extract::State, Json};
use sea_orm::{ColumnTrait, Condition};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{is_valid_id, BasicModel, Network, Token, TokenColumn},
	App, IdPrefix,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	id: Option<String>,
	network: String,
	chain_id: Option<u64>,
	name: String,
	symbol: String,
	address: Option<String>,
	decimals: u16,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<Json<Token>> {
	let chain_id = payload.chain_id.unwrap_or_default();
	let address = payload.address.unwrap_or_default();

	// check that id is valid
	if let Some(id) = payload.id.clone() {
		if !is_valid_id(&id, IdPrefix::Token) || Token::get_by_id(app.db(), &id).await?.is_some() {
			return Err(ServerError::InvalidParam { field: "id".to_string(), value: id });
		}
	}

	// fetch network
	let network =
		Network::get_by_id(app.db(), &payload.network).await?.ok_or(ServerError::InvalidParam {
			field: "network".to_string(),
			value: payload.network,
		})?;

	// check for duplicate network + chain_id + address
	if !Token::get_all_where(
		app.db(),
		Condition::all()
			.add(TokenColumn::NetworkId.eq(network.network_id))
			.add(TokenColumn::ChainId.eq(chain_id))
			.add(TokenColumn::Address.eq(address.clone())),
	)
	.await?
	.is_empty()
	{
		return Err(ServerError::Duplicate { field: "address".to_string(), value: address });
	}

	// create new
	let token_id = Token::create(
		app.db(),
		Token::new_model(
			payload.id,
			network.network_id,
			chain_id as i64,
			&payload.name,
			&payload.symbol,
			&address,
			payload.decimals as i16,
		),
	)
	.await?;

	// return newly created
	Ok(Token::get(app.db(), token_id).await?.unwrap().into())
}
