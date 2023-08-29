use eyre::Result;
use sea_orm::{
	entity::{prelude::*, *},
	ConnectionTrait, FromQueryResult, QuerySelect,
};
use sea_orm_migration::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
	models::{db::entity_tag, BasicModel, EntityTagColumn, PrimaryId, PrimaryIds},
	utils, IdPrefix, RiskLevel,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub tag_id: PrimaryId,
	pub id: String,
	pub name: String,
	pub risk_level: RiskLevel,
	#[sea_orm(nullable)]
	#[serde(skip_serializing)]
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,

	#[sea_orm(ignore)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub entities: Option<Vec<String>>,
}

impl From<Vec<Model>> for PrimaryIds {
	fn from(m: Vec<Model>) -> PrimaryIds {
		let mut ids: Vec<PrimaryId> = m.iter().map(|m| m.tag_id).collect();

		ids.sort_unstable();
		ids.dedup();

		PrimaryIds(ids)
	}
}

#[derive(Clone, FromQueryResult)]
pub struct JoinedModel {
	pub tag_id: PrimaryId,
	pub id: String,
	pub name: String,
	pub risk_level: RiskLevel,
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,
	pub entity_id: PrimaryId,
}

impl From<Vec<JoinedModel>> for PrimaryIds {
	fn from(m: Vec<JoinedModel>) -> PrimaryIds {
		let mut ids: Vec<PrimaryId> = m.iter().map(|m| m.tag_id).collect();

		ids.sort_unstable();
		ids.dedup();

		PrimaryIds(ids)
	}
}

impl From<JoinedModel> for Model {
	fn from(m: JoinedModel) -> Model {
		Model {
			tag_id: m.tag_id,
			id: m.id,
			name: m.name,
			risk_level: m.risk_level,
			updated_at: m.updated_at,
			created_at: m.created_at,
			entities: None,
		}
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanitizedTag {
	pub id: String,
	pub name: String,
	pub risk_level: RiskLevel,
}

impl From<Model> for SanitizedTag {
	fn from(m: Model) -> SanitizedTag {
		SanitizedTag { id: m.id, name: m.name, risk_level: m.risk_level }
	}
}

pub use ActiveModel as TagActiveModel;
pub use JoinedModel as JoinedTag;
pub use Model as Tag;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "entity_tag::Entity",
		from = "Column::TagId",
		to = "EntityTagColumn::TagId"
	)]
	EntityTag,
}

impl ActiveModelBehavior for ActiveModel {}

impl BasicModel for Model {
	type ActiveModel = ActiveModel;
}

impl Model {
	pub fn new_model(id: Option<String>, name: &str, risk_level: RiskLevel) -> ActiveModel {
		ActiveModel {
			id: Set(id.unwrap_or(utils::new_unique_id(IdPrefix::Tag))),
			name: Set(name.to_string()),
			risk_level: Set(risk_level),
			..Default::default()
		}
	}

	pub async fn get_by_name<C>(c: &C, name: &str) -> Result<Option<Self>>
	where
		C: ConnectionTrait,
	{
		Ok(Entity::find()
			.filter(Condition::all().add(
				Expr::expr(Func::lower(Expr::col(Column::Name))).eq(name.trim().to_lowercase()),
			))
			.one(c)
			.await?)
	}

	pub async fn get_all_by_entity_ids<C>(c: &C, entity_ids: PrimaryIds) -> Result<Vec<JoinedModel>>
	where
		C: ConnectionTrait,
	{
		Ok(Entity::find()
			.column_as(EntityTagColumn::EntityId, "entity_id")
			.join(JoinType::LeftJoin, Relation::EntityTag.def())
			.filter(EntityTagColumn::EntityId.is_in(entity_ids))
			.into_model::<JoinedModel>()
			.all(c)
			.await?)
	}
}
