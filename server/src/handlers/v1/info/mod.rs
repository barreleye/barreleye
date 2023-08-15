use axum::{routing::get, Router};
use std::sync::Arc;

use barreleye_common::App;

mod get;

pub fn get_routes() -> Router<Arc<App>> {
	Router::new().route("/", get(get::handler))
}
