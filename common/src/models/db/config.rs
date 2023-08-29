use derive_more::Display;
use eyre::Result;
use regex::Regex;
use sea_orm::{entity::prelude::*, Condition, ConnectionTrait, Set};
use sea_orm_migration::prelude::{Expr, OnConflict};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use crate::{models::PrimaryId, utils, BlockHeight};

// Things to keep in mind when defining configs:
// 0. stick to similar format: "title_a1_b2_c3"
// 1. one letter per object: "network" => "n"
// 2. no similar prefix (has to do with "LIKE" selection syntax in `adjust_filter()`) bad:
//    "title_a1_b2" & "title_a1_b2_c3" good: "title_a1_b2" & "diff_title_a1_b2_c3"
#[derive(Ord, PartialOrd, Display, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ConfigKey {
	#[display(fmt = "primary")]
	Primary,
	#[display(fmt = "indexer_sync_tail_n{_0}")]
	IndexerSyncTail(PrimaryId),
	#[display(fmt = "indexer_sync_chunk_n{_0}_b{_1}")]
	IndexerSyncChunk(PrimaryId, BlockHeight),
	#[display(fmt = "indexer_sync_progress_n{_0}")]
	IndexerSyncProgress(PrimaryId),
	#[display(fmt = "indexer_process_tail_n{_0}")]
	IndexerProcessTail(PrimaryId),
	#[display(fmt = "indexer_process_chunk_n{_0}_b{_1}")]
	IndexerProcessChunk(PrimaryId, BlockHeight),
	#[display(fmt = "indexer_process_module_n{_0}_m{_1}")]
	IndexerProcessModule(PrimaryId, u16),
	#[display(fmt = "indexer_process_module_done_n{_0}_m{_1}")]
	IndexerProcessModuleDone(PrimaryId, u16),
	#[display(fmt = "indexer_process_progress_n{_0}")]
	IndexerProcessProgress(PrimaryId),
	#[display(fmt = "indexer_link_n{_0}_a{_1}")]
	IndexerLink(PrimaryId, PrimaryId),
	#[display(fmt = "block_height_n{_0}")]
	BlockHeight(PrimaryId),
	#[display(fmt = "networks_updated")]
	NetworksUpdated,
	#[display(fmt = "newly_added_address_n{_0}_a{_1}")]
	NewlyAddedAddress(PrimaryId, PrimaryId),
}

impl From<String> for ConfigKey {
	fn from(s: String) -> Self {
		let re = Regex::new(r"(\d+)").unwrap();

		let template = re.replace_all(&s, "{}");
		let n = re.find_iter(&s).filter_map(|n| n.as_str().parse().ok()).collect::<Vec<i64>>();

		match template.to_string().as_str() {
			"primary" => Self::Primary,
			"indexer_sync_tail_n{}" if n.len() == 1 => Self::IndexerSyncTail(n[0]),
			"indexer_sync_chunk_n{}_b{}" if n.len() == 2 => {
				Self::IndexerSyncChunk(n[0], n[1] as BlockHeight)
			}
			"indexer_sync_progress_n{}" if n.len() == 1 => Self::IndexerSyncProgress(n[0]),
			"indexer_process_tail_n{}" if n.len() == 1 => Self::IndexerProcessTail(n[0]),
			"indexer_process_chunk_n{}_b{}" if n.len() == 2 => {
				Self::IndexerProcessChunk(n[0], n[1] as BlockHeight)
			}
			"indexer_process_module_n{}_m{}" if n.len() == 2 => {
				Self::IndexerProcessModule(n[0], n[1] as u16)
			}
			"indexer_process_module_done_n{}_m{}" if n.len() == 2 => {
				Self::IndexerProcessModuleDone(n[0], n[1] as u16)
			}
			"indexer_process_progress_n{}" if n.len() == 1 => Self::IndexerProcessProgress(n[0]),
			"indexer_link_n{}_a{}" if n.len() == 2 => Self::IndexerLink(n[0], n[1]),
			"block_height_n{}" if n.len() == 1 => Self::BlockHeight(n[0]),
			"networks_updated" => Self::NetworksUpdated,
			"newly_added_address_n{}_a{}" if n.len() == 2 => Self::NewlyAddedAddress(n[0], n[1]),
			_ => panic!("no match in From<String> for ConfigKey: {s:?}"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_config_key_str() {
		let config_keys = HashMap::from([
			(ConfigKey::Primary, "primary"),
			(ConfigKey::IndexerSyncTail(123), "indexer_sync_tail_n123"),
			(ConfigKey::IndexerSyncChunk(123, 456), "indexer_sync_chunk_n123_b456"),
			(ConfigKey::IndexerSyncProgress(123), "indexer_sync_progress_n123"),
			(ConfigKey::IndexerProcessTail(123), "indexer_process_tail_n123"),
			(ConfigKey::IndexerProcessChunk(123, 456), "indexer_process_chunk_n123_b456"),
			(ConfigKey::IndexerProcessModule(123, 456), "indexer_process_module_n123_m456"),
			(
				ConfigKey::IndexerProcessModuleDone(123, 456),
				"indexer_process_module_done_n123_m456",
			),
			(ConfigKey::IndexerProcessProgress(123), "indexer_process_progress_n123"),
			(ConfigKey::IndexerLink(123, 456), "indexer_link_n123_a456"),
			(ConfigKey::BlockHeight(123), "block_height_n123"),
			(ConfigKey::NetworksUpdated, "networks_updated"),
			(ConfigKey::NewlyAddedAddress(123, 456), "newly_added_address_n123_a456"),
		]);

		for (config_key, config_key_str) in config_keys.into_iter() {
			assert_eq!(config_key.to_string(), config_key_str);
			assert_eq!(Into::<ConfigKey>::into(config_key_str.to_string()), config_key);
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "configs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub config_id: PrimaryId,
	pub key: String,
	pub value: String,
	#[serde(skip_serializing)]
	pub updated_at: DateTime,
	pub created_at: DateTime,
}

#[derive(Debug)]
pub struct Value<T: for<'a> Deserialize<'a>> {
	pub value: T,
	pub updated_at: DateTime,
	pub created_at: DateTime,
}

pub use ActiveModel as ConfigActiveModel;
pub use Model as Config;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
	pub async fn set<C, T>(c: &C, key: ConfigKey, value: T) -> Result<()>
	where
		C: ConnectionTrait,
		T: Serialize,
	{
		Entity::insert(ActiveModel {
			key: Set(key.to_string()),
			value: Set(json!(value).to_string()),
			updated_at: Set(utils::now()),
			..Default::default()
		})
		.on_conflict(
			OnConflict::column(Column::Key)
				.update_columns([Column::Value, Column::UpdatedAt])
				.to_owned(),
		)
		.exec(c)
		.await?;

		Ok(())
	}

	pub async fn set_where<C, T>(
		c: &C,
		key: ConfigKey,
		value: T,
		where_value: Value<T>,
	) -> Result<bool>
	where
		C: ConnectionTrait,
		T: Serialize + for<'a> Deserialize<'a>,
	{
		let update_result = Entity::update_many()
			.col_expr(Column::Value, Expr::value(json!(value).to_string()))
			.col_expr(Column::UpdatedAt, Expr::value(utils::now()))
			.filter(Column::Key.eq(key.to_string()))
			.filter(Column::Value.eq(json!(where_value.value).to_string()))
			.filter(Column::UpdatedAt.eq(where_value.updated_at))
			.exec(c)
			.await?;

		Ok(update_result.rows_affected == 1)
	}

	pub async fn set_many<C, T>(c: &C, values: HashMap<ConfigKey, T>) -> Result<()>
	where
		C: ConnectionTrait,
		T: Serialize,
	{
		let insert_data = values
			.into_iter()
			.map(|(key, value)| ActiveModel {
				key: Set(key.to_string()),
				value: Set(json!(value).to_string()),
				updated_at: Set(utils::now()),
				..Default::default()
			})
			.collect::<Vec<ActiveModel>>();

		Entity::insert_many(insert_data)
			.on_conflict(
				OnConflict::column(Column::Key)
					.update_columns([Column::Value, Column::UpdatedAt])
					.to_owned(),
			)
			.exec(c)
			.await?;

		Ok(())
	}

	pub async fn get<C, T>(c: &C, key: ConfigKey) -> Result<Option<Value<T>>>
	where
		C: ConnectionTrait,
		T: for<'a> Deserialize<'a>,
	{
		Ok(Entity::find().filter(Column::Key.eq(key.to_string())).one(c).await?.map(|m| Value {
			value: serde_json::from_str(&m.value).unwrap(),
			updated_at: m.updated_at,
			created_at: m.created_at,
		}))
	}

	pub async fn get_many<C, T>(
		c: &C,
		mut keys: Vec<ConfigKey>,
	) -> Result<HashMap<ConfigKey, Value<T>>>
	where
		C: ConnectionTrait,
		T: for<'a> Deserialize<'a>,
	{
		keys.sort_unstable();
		keys.dedup();

		Ok(Entity::find()
			.filter(Self::adjust_filter(keys))
			.all(c)
			.await?
			.into_iter()
			.map(|m| {
				(
					m.key.into(),
					Value {
						value: serde_json::from_str(&m.value).unwrap(),
						updated_at: m.updated_at,
						created_at: m.created_at,
					},
				)
			})
			.collect())
	}

	pub async fn delete<C>(c: &C, key: ConfigKey) -> Result<()>
	where
		C: ConnectionTrait,
	{
		Entity::delete_many().filter(Column::Key.eq(key.to_string())).exec(c).await?;
		Ok(())
	}

	pub async fn delete_many<C>(c: &C, mut keys: Vec<ConfigKey>) -> Result<()>
	where
		C: ConnectionTrait,
	{
		keys.sort_unstable();
		keys.dedup();

		Entity::delete_many()
			.filter(Column::Key.is_in(keys.into_iter().map(|k| k.to_string())))
			.exec(c)
			.await?;
		Ok(())
	}

	pub async fn delete_all_by_keywords<C>(c: &C, keywords: Vec<String>) -> Result<()>
	where
		C: ConnectionTrait,
	{
		Entity::delete_many().filter(Self::get_keyword_conditions(keywords)).exec(c).await?;
		Ok(())
	}

	fn adjust_filter(keys: Vec<ConfigKey>) -> Condition {
		let mut condition = Condition::any();

		// only match zeros: `example_a100_b0_c123` => `example_a100_b%_c123`
		let r = Regex::new(r"_([a-z])0").unwrap();

		for key in keys.into_iter().map(|k| k.to_string()) {
			let adjusted_key = r.replace(&key, "_$1%");
			condition = condition.add(if adjusted_key.contains('%') {
				Column::Key.like(adjusted_key.clone())
			} else {
				Column::Key.eq(&key)
			});
		}

		condition
	}

	fn get_keyword_conditions(keywords: Vec<String>) -> Condition {
		let mut condition = Condition::any();

		for keyword in keywords.into_iter() {
			condition = condition.add(Column::Key.like(&format!("%_{keyword}_%")));
			condition = condition.add(Column::Key.like(&format!("%_{keyword}")));
		}

		condition
	}
}
