use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{is_valid_id, BasicModel, Tag},
	App, IdPrefix, RiskLevel,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	id: Option<String>,
	name: String,
	risk_level: RiskLevel,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<Json<Tag>> {
	// check that id is valid
	if let Some(id) = payload.id.clone() {
		if !is_valid_id(&id, IdPrefix::Tag) || Tag::get_by_id(app.db(), &id).await?.is_some() {
			return Err(ServerError::InvalidParam { field: "id".to_string(), value: id });
		}
	}

	// check for duplicate name
	if Tag::get_by_name(app.db(), &payload.name).await?.is_some() {
		return Err(ServerError::Duplicate { field: "name".to_string(), value: payload.name });
	}

	// create new
	let tag_id =
		Tag::create(app.db(), Tag::new_model(payload.id, &payload.name, payload.risk_level))
			.await?;

	// return newly created
	Ok(Tag::get(app.db(), tag_id).await?.unwrap().into())
}
