use eyre::Result;
use sea_orm::{
	entity::{prelude::*, *},
	ConnectionTrait,
};
use sea_orm_migration::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::{
	db::{entity, tag},
	BasicModel, EntityColumn, PrimaryId, PrimaryIds, TagColumn,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "entity_tags")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub entity_id: PrimaryId,
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub tag_id: PrimaryId,
	pub created_at: DateTime,
}

pub use ActiveModel as EntityTagActiveModel;
pub use Model as EntityTag;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "entity::Entity",
		from = "Column::EntityId",
		to = "EntityColumn::EntityId"
	)]
	Entity,
	#[sea_orm(belongs_to = "tag::Entity", from = "Column::TagId", to = "TagColumn::TagId")]
	Tag,
}

impl ActiveModelBehavior for ActiveModel {}

impl BasicModel for Model {
	type ActiveModel = ActiveModel;
}

impl Model {
	pub fn new_model(entity_id: PrimaryId, tag_id: PrimaryId) -> ActiveModel {
		ActiveModel { entity_id: Set(entity_id), tag_id: Set(tag_id), ..Default::default() }
	}

	pub async fn create_many<C>(c: &C, data: Vec<ActiveModel>) -> Result<(PrimaryId, PrimaryId)>
	where
		C: ConnectionTrait,
	{
		let insert_result = Entity::insert_many(data)
			.on_conflict(
				OnConflict::columns([Column::EntityId, Column::TagId]).do_nothing().to_owned(),
			)
			.exec(c)
			.await?;

		Ok(insert_result.last_insert_id)
	}

	pub async fn delete_not_included_tags<C>(
		c: &C,
		entity_id: PrimaryId,
		tag_ids: PrimaryIds,
	) -> Result<u64>
	where
		C: ConnectionTrait,
	{
		let res = Entity::delete_many()
			.filter(Column::EntityId.eq(entity_id))
			.filter(Column::TagId.is_not_in(tag_ids))
			.exec(c)
			.await?;

		Ok(res.rows_affected)
	}
}
