use clickhouse::Row;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{
	models::{warehouse::transfer::TABLE as TRANSFERS_TABLE, PrimaryId, PrimaryIds},
	warehouse::Warehouse,
	BlockHeight,
};

pub static TABLE: &str = "links";

// @TODO ideally this wouldn't have to be wrapped
#[derive(PartialEq, Eq, Hash, Debug, Clone, Row, Serialize, Deserialize)]
pub struct LinkUuid(#[serde(with = "clickhouse::serde::uuid")] pub Uuid);

#[derive(PartialEq, Eq, Hash, Debug, Clone, Row, Serialize, Deserialize)]
pub struct Model {
	pub network_id: u64,
	pub block_height: u64,
	pub from_address: String,
	pub to_address: String,
	pub transfer_uuids: Vec<LinkUuid>,
	pub created_at: u32,
}

pub use Model as Link;

impl Model {
	pub fn new(
		network_id: PrimaryId,
		block_height: u64,
		from_address: &str,
		to_address: &str,
		transfer_uuids: Vec<LinkUuid>,
		created_at: u32,
	) -> Self {
		Self {
			network_id: network_id as u64,
			block_height,
			from_address: from_address.to_string(),
			to_address: to_address.to_string(),
			transfer_uuids,
			created_at,
		}
	}

	pub async fn create_many(warehouse: &Warehouse, models: Vec<Self>) -> Result<()> {
		let mut insert = warehouse.get().insert(TABLE)?;
		for model in models.into_iter() {
			insert.write(&model).await?;
		}

		Ok(insert.end().await?)
	}

	pub async fn get_all_by_addresses(
		warehouse: &Warehouse,
		mut addresses: Vec<String>,
	) -> Result<Vec<Self>> {
		addresses.sort_unstable();
		addresses.dedup();

		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SELECT *
					FROM {TABLE}
					WHERE to_address IN ?
                "#
			))
			.bind(addresses)
			.fetch_all::<Model>()
			.await?)
	}

	pub async fn get_all_disinct_by_addresses(
		warehouse: &Warehouse,
		mut addresses: Vec<String>,
	) -> Result<Vec<Self>> {
		addresses.sort_unstable();
		addresses.dedup();

		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SELECT DISTINCT ON (network_id, from_address) *
					FROM {TABLE}
					WHERE to_address IN ?
					ORDER BY LENGTH(transfer_uuids) ASC
                "#
			))
			.bind(addresses)
			.fetch_all::<Model>()
			.await?)
	}

	pub async fn get_all_to_seed_blocks(
		warehouse: &Warehouse,
		network_id: PrimaryId,
		(block_height_min, block_height_max): (BlockHeight, BlockHeight),
	) -> Result<Vec<Self>> {
		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SELECT *
					FROM {TABLE}
					WHERE network_id = ? AND to_address IN (
					    SELECT from_address
					    FROM {TRANSFERS_TABLE}
					    WHERE
							network_id = ? AND
							length(from_address) > 0 AND
							length(to_address) > 0 AND
							block_height >= ? AND
							block_height <= ?
					)
                "#
			))
			.bind(network_id)
			.bind(network_id)
			.bind(block_height_min)
			.bind(block_height_max)
			.fetch_all::<Model>()
			.await?)
	}

	pub async fn delete_all_by_sources(
		warehouse: &Warehouse,
		sources: HashMap<PrimaryId, HashSet<String>>, // network_id -> addresses
	) -> Result<()> {
		if !sources.is_empty() {
			warehouse
				.get()
				.query(&format!(
					r#"
						SET allow_experimental_lightweight_delete = true;
						DELETE FROM {TABLE} WHERE {}
					"#,
					Self::get_network_id_address_tuples(sources, "from_address"),
				))
				.execute()
				.await?;
		}

		Ok(())
	}

	pub async fn delete_all_by_network_id(
		warehouse: &Warehouse,
		network_ids: PrimaryIds,
	) -> Result<()> {
		Ok(warehouse
			.get()
			.query(&format!(
				r#"
					SET allow_experimental_lightweight_delete = true;
					DELETE FROM {TABLE} WHERE network_id IN ?
                "#
			))
			.bind(network_ids.into_iter().collect::<Vec<PrimaryId>>())
			.execute()
			.await?)
	}

	pub async fn delete_all_by_newly_added_addresses(
		warehouse: &Warehouse,
		targets: HashMap<PrimaryId, HashSet<String>>, // network_id -> addresses
	) -> Result<()> {
		// when a new entity address is added (let's call it X),
		// we need to clean up this model's table because some entries might contain X
		// *in the middle* of their `transfer_uuids` chain.
		//
		// and we don't want that because every upstream response should point to
		// *the closest* labeled entity.
		//
		// so we have to break up those "chains" for every X that is added. this is what
		// this function does.
		//
		// steps:
		// 1. find all links where `to_address` is in (targets)
		// 2. gather the last elements of those `transfer_uuids` into an array
		// 3. delete all link records that contain those uuids in the middle of `transfer_uuids`
		//    meaning not first, because it's ok if target is in the `from_address`
		//    but also not last, because it's ok if target is in the `to_address`
		//    (in the middle = labeled entity is in the middle of the links chain)

		if !targets.is_empty() {
			warehouse
				.get()
				.query(&format!(
					r#"
						SET allow_experimental_lightweight_delete = true;
						DELETE FROM {TABLE}
						WHERE
							length(transfer_uuids) > 2 AND
							hasAny(
								arraySlice(transfer_uuids, 2, -1),
								(
									SELECT DISTINCT groupArray(transfer_uuids[-1])
									FROM {TABLE}
									WHERE length(transfer_uuids) > 0 AND {}
								)
							)
					"#,
					Self::get_network_id_address_tuples(targets, "to_address"),
				))
				.execute()
				.await?;
		}

		Ok(())
	}

	fn get_network_id_address_tuples(
		map: HashMap<PrimaryId, HashSet<String>>,
		field: &str,
	) -> String {
		map.into_iter()
			.map(|(network_id, addresses)| {
				let escaped_addresses = addresses
					.into_iter()
					.map(|a| format!("'{}'", a.replace('\\', "\\\\").replace('\'', "\\'")))
					.collect::<Vec<String>>()
					.join(",");

				format!("(network_id = {network_id} AND {field} IN ({escaped_addresses}))")
			})
			.collect::<Vec<String>>()
			.join(" OR ")
	}
}
