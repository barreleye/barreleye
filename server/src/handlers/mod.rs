use axum::Router;
use std::sync::Arc;

use barreleye_common::App;

mod v1;

pub fn get_routes() -> Router<Arc<App>> {
	Router::new().nest("/v1", v1::get_routes())
}
