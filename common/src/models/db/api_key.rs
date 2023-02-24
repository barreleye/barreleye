use eyre::Result;
use sea_orm::{
	entity::{prelude::*, *},
	ConnectionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
	models::{BasicModel, PrimaryId},
	utils, IdPrefix,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "api_keys")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub api_key_id: PrimaryId,
	pub id: String,
	#[serde(skip_serializing)]
	pub uuid: Uuid,
	pub is_active: bool,
	#[sea_orm(nullable)]
	#[serde(skip_serializing)]
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,

	#[sea_orm(ignore)]
	pub key: String, // abbreviated `uuid` used in responses
}

pub use ActiveModel as ApiKeyActiveModel;
pub use Model as ApiKey;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl BasicModel for Model {
	type ActiveModel = ActiveModel;
}

impl Model {
	pub fn new_model() -> ActiveModel {
		ActiveModel {
			id: Set(utils::new_unique_id(IdPrefix::ApiKey)),
			uuid: Set(utils::new_uuid()),
			is_active: Set(true),
			..Default::default()
		}
	}

	pub async fn get_by_uuid<C>(c: &C, uuid: &Uuid) -> Result<Option<Self>>
	where
		C: ConnectionTrait,
	{
		Ok(Entity::find().filter(Column::Uuid.eq(*uuid)).one(c).await?)
	}

	pub fn format(&self) -> Self {
		Self { key: self.uuid.to_string()[..4].to_string(), ..self.clone() }
	}
}
