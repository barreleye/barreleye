use eyre::Result;
use sea_orm::{
	entity::prelude::*,
	sea_query::{func::Func, Expr},
	Condition, ConnectionTrait, FromQueryResult, QuerySelect, Set,
};
use sea_orm_migration::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
	models::{db::entity_tag, BasicModel, EntityTagColumn, PrimaryId, PrimaryIds, SoftDeleteModel},
	utils, IdPrefix,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "entities")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub entity_id: PrimaryId,
	pub id: String,
	#[sea_orm(nullable)]
	pub name: Option<String>,
	pub description: String,
	pub data: Json,
	#[serde(skip_serializing)]
	pub is_deleted: bool,
	#[sea_orm(nullable)]
	#[serde(skip_serializing)]
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,

	#[sea_orm(ignore)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Vec<String>>,
	#[sea_orm(ignore)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub addresses: Option<Vec<String>>,
}

impl From<Vec<Model>> for PrimaryIds {
	fn from(m: Vec<Model>) -> PrimaryIds {
		let mut ids: Vec<PrimaryId> = m.iter().map(|m| m.entity_id).collect();

		ids.sort_unstable();
		ids.dedup();

		PrimaryIds(ids)
	}
}

#[derive(Clone, FromQueryResult)]
pub struct JoinedModel {
	pub entity_id: PrimaryId,
	pub id: String,
	pub name: Option<String>,
	pub description: String,
	pub data: Json,
	pub is_deleted: bool,
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,
	pub tag_id: PrimaryId,
}

impl From<JoinedModel> for Model {
	fn from(m: JoinedModel) -> Model {
		Model {
			entity_id: m.entity_id,
			id: m.id,
			name: m.name,
			description: m.description,
			data: m.data,
			is_deleted: m.is_deleted,
			updated_at: m.updated_at,
			created_at: m.created_at,
			tags: None,
			addresses: None,
		}
	}
}

impl From<Vec<JoinedModel>> for PrimaryIds {
	fn from(m: Vec<JoinedModel>) -> PrimaryIds {
		let mut ids: Vec<PrimaryId> = m.iter().map(|m| m.entity_id).collect();

		ids.sort_unstable();
		ids.dedup();

		PrimaryIds(ids)
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanitizedEntity {
	pub id: String,
	pub name: Option<String>,
	pub description: String,
	pub data: Json,
	pub tags: Option<Vec<String>>,
}

impl From<Model> for SanitizedEntity {
	fn from(m: Model) -> SanitizedEntity {
		SanitizedEntity {
			id: m.id,
			name: m.name,
			description: m.description,
			data: m.data,
			tags: m.tags,
		}
	}
}

pub use ActiveModel as LabeledEntityActiveModel;
pub use JoinedModel as JoinedEntity;
pub use Model as LabeledEntity;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "entity_tag::Entity",
		from = "Column::EntityId",
		to = "EntityTagColumn::EntityId"
	)]
	EntityTag,
}

impl ActiveModelBehavior for ActiveModel {}

impl BasicModel for Model {
	type ActiveModel = ActiveModel;
}

impl SoftDeleteModel for Model {
	type ActiveModel = ActiveModel;
}

impl Model {
	pub fn new_model(
		id: Option<String>,
		name: Option<String>,
		description: &str,
		data: Option<Json>,
	) -> ActiveModel {
		ActiveModel {
			id: Set(id.unwrap_or(utils::new_unique_id(IdPrefix::Entity))),
			name: Set(name),
			description: Set(description.to_string()),
			data: Set(data.unwrap_or(json!({}))),
			is_deleted: Set(false),
			..Default::default()
		}
	}

	pub async fn get_by_name<C>(c: &C, name: &str, is_deleted: Option<bool>) -> Result<Option<Self>>
	where
		C: ConnectionTrait,
	{
		let mut q =
			Entity::find().filter(Condition::all().add(
				Expr::expr(Func::lower(Expr::col(Column::Name))).eq(name.trim().to_lowercase()),
			));

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Column::IsDeleted.eq(is_deleted))
		}

		Ok(q.one(c).await?)
	}

	pub async fn get_all_by_entity_ids<C>(
		c: &C,
		entity_ids: PrimaryIds,
		is_deleted: Option<bool>,
	) -> Result<Vec<Self>>
	where
		C: ConnectionTrait,
	{
		let mut q = Entity::find().filter(Column::EntityId.is_in(entity_ids));

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Column::IsDeleted.eq(is_deleted))
		}

		Ok(q.all(c).await?)
	}

	pub async fn get_all_by_tag_ids<C>(
		c: &C,
		tag_ids: PrimaryIds,
		is_deleted: Option<bool>,
	) -> Result<Vec<JoinedModel>>
	where
		C: ConnectionTrait,
	{
		let mut q = Entity::find()
			.column_as(EntityTagColumn::TagId, "tag_id")
			.join(JoinType::LeftJoin, Relation::EntityTag.def())
			.filter(EntityTagColumn::TagId.is_in(tag_ids));

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Column::IsDeleted.eq(is_deleted))
		}

		Ok(q.into_model::<JoinedModel>().all(c).await?)
	}
}
