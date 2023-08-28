use std::collections::HashMap;

use crate::{errors::ServerError, ServerResult};
use barreleye_common::{
	models::{is_valid_id, PrimaryId},
	IdPrefix,
};

pub fn extract_primary_ids(
	field: &str,
	mut ids: Vec<String>,
	id_prefix: IdPrefix,
	map: HashMap<String, PrimaryId>,
) -> ServerResult<Vec<PrimaryId>> {
	if !ids.is_empty() {
		ids.sort_unstable();
		ids.dedup();

		let invalid_ids = ids
			.into_iter()
			.filter_map(|id| {
				if !map.contains_key(&id) && is_valid_id(&id, id_prefix.clone()) {
					Some(id)
				} else {
					None
				}
			})
			.collect::<Vec<String>>();

		if !invalid_ids.is_empty() {
			return Err(ServerError::InvalidValues {
				field: field.to_string(),
				values: invalid_ids.join(", "),
			});
		}

		return Ok(map.into_values().collect());
	}

	Ok(vec![])
}
