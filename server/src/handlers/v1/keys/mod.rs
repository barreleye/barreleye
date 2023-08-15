use axum::{
	routing::{delete, get, post, put},
	Router,
};
use std::sync::Arc;

use barreleye_common::App;

mod create;
mod delete;
mod get;
mod list;
mod update;

pub fn get_routes() -> Router<Arc<App>> {
	Router::new()
		.route("/", post(create::handler))
		.route("/", get(list::handler))
		.route("/:id", get(get::handler))
		.route("/:id", put(update::handler))
		.route("/:id", delete(delete::handler))
}
