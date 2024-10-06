use axum::{extract::State, http::StatusCode, Json};
use sea_orm::ColumnTrait;
use serde::Deserialize;
use std::{collections::HashSet, sync::Arc};

use crate::ServerResult;
use barreleye_common::{
	models::{
		set, Address, AddressActiveModel, AddressColumn, BasicModel, Entity, EntityActiveModel,
		EntityColumn, PrimaryId,
	},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	entities: HashSet<String>,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<StatusCode> {
	// exit if no input
	if payload.entities.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// get all entities
	let all_entities =
		Entity::get_all_where(app.db(), EntityColumn::Id.is_in(payload.entities)).await?;

	// proceed only when there's something to delete
	if all_entities.is_empty() {
		return Ok(StatusCode::NO_CONTENT);
	}

	// soft-delete all associated addresses
	let all_entity_ids = all_entities.iter().map(|e| e.entity_id).collect::<Vec<PrimaryId>>();
	Address::update_all_where(
		app.db(),
		AddressColumn::EntityId.is_in(all_entity_ids.clone()),
		AddressActiveModel { is_deleted: set(true), ..Default::default() },
	)
	.await?;

	// soft-delete all entities
	Entity::update_all_where(
		app.db(),
		EntityColumn::EntityId.is_in(all_entity_ids),
		EntityActiveModel { is_deleted: set(true), ..Default::default() },
	)
	.await?;

	Ok(StatusCode::NO_CONTENT)
}
