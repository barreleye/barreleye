use axum::{extract::State, http::StatusCode, Json};
use sea_orm::ColumnTrait;
use serde::Deserialize;
use std::{collections::HashSet, sync::Arc};

use crate::ServerResult;
use barreleye_common::{
	models::{BasicModel, PrimaryId, Token, TokenColumn},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	tokens: HashSet<String>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<StatusCode> {
	// exit if no input
	if payload.tokens.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// get all tokens
	let all_tokens =
		Token::get_all_where(app.db(), TokenColumn::Id.is_in(payload.tokens))
			.await?;

	// proceed only when there's something to delete
	if all_tokens.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// delete all associated tokens
	Token::delete_all_where(
		app.db(),
		TokenColumn::TokenId.is_in(
			all_tokens.iter().map(|t| t.token_id).collect::<Vec<PrimaryId>>(),
		),
	)
	.await?;

	Ok(StatusCode::NO_CONTENT)
}
