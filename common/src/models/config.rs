use derive_more::Display;
use eyre::Result;
use sea_orm::{entity::prelude::*, Set};
use sea_orm_migration::prelude::OnConflict;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{models::PrimaryId, utils, Db};

#[derive(Display, Debug, Clone)]
pub enum ConfigKey {
	#[display(fmt = "leader")]
	Leader,
	#[display(fmt = "label_fetched_{}", "_0")]
	LabelFetched(PrimaryId),
	#[display(fmt = "last_saved_block_{}", "_0")]
	LastSavedBlock(u64),
	#[display(fmt = "block_height_{}", "_0")]
	BlockHeight(u64),
}

impl From<ConfigKey> for String {
	fn from(config_key: ConfigKey) -> String {
		config_key.to_string()
	}
}

#[derive(
	Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel,
)]
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

pub struct Value<T: for<'a> Deserialize<'a>> {
	pub value: T,
	pub updated_at: DateTime,
	pub created_at: DateTime,
}

pub use ActiveModel as ConfigActiveModel;
pub use Model as Config;

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
	fn def(&self) -> RelationDef {
		panic!("No RelationDef")
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
	pub async fn set<T>(db: &Db, key: ConfigKey, value: T) -> Result<()>
	where
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
		.exec(db.get())
		.await?;

		Ok(())
	}

	pub async fn get<T>(db: &Db, key: ConfigKey) -> Result<Option<Value<T>>>
	where
		T: for<'a> Deserialize<'a>,
	{
		Ok(Entity::find()
			.filter(Column::Key.eq(key.to_string()))
			.one(db.get())
			.await?
			.map(|m| Value {
				value: serde_json::from_str(&m.value).unwrap(),
				updated_at: m.updated_at,
				created_at: m.created_at,
			}))
	}
}