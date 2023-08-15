use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{
		set, Address, AddressActiveModel, AddressColumn, BasicModel, Entity, EntityActiveModel,
		SoftDeleteModel,
	},
	App,
};
use sea_orm::ColumnTrait;

pub async fn handler(
	State(app): State<Arc<App>>,
	Path(entity_id): Path<String>,
) -> ServerResult<StatusCode> {
	if let Some(entity) = Entity::get_existing_by_id(app.db(), &entity_id).await? {
		// soft-delete all associated addresses
		Address::update_all_where(
			app.db(),
			AddressColumn::EntityId.eq(entity.entity_id),
			AddressActiveModel { is_deleted: set(true), ..Default::default() },
		)
		.await?;

		// soft-delete entity
		Entity::update_by_id(
			app.db(),
			&entity_id,
			EntityActiveModel { is_deleted: set(true), ..Default::default() },
		)
		.await?;

		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(ServerError::NotFound)
	}
}
