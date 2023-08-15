use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{BasicModel, Tag},
	App,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
	name: String,
}

pub async fn handler(
	State(app): State<Arc<App>>,
	Json(payload): Json<Payload>,
) -> ServerResult<Json<Tag>> {
	// check for duplicate name
	if Tag::get_by_name(app.db(), &payload.name).await?.is_some() {
		return Err(ServerError::Duplicate { field: "name".to_string(), value: payload.name });
	}

	// create new
	let tag_id = Tag::create(app.db(), Tag::new_model(&payload.name)).await?;

	// return newly created
	Ok(Tag::get(app.db(), tag_id).await?.unwrap().into())
}
