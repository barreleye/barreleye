use async_trait::async_trait;
use eyre::Result;
use sea_orm::{
	entity::{prelude::*, *},
	ConnectionTrait,
};
use sea_orm_migration::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
	models::{db::entity, BasicModel, EntityColumn, PrimaryId, PrimaryIds, SoftDeleteModel},
	utils, IdPrefix,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "addresses")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub address_id: PrimaryId,
	#[serde(skip_serializing)]
	pub entity_id: PrimaryId,
	#[serde(skip_serializing)]
	pub network_id: PrimaryId,
	pub network: String,
	pub id: String,
	pub address: String,
	pub description: String,
	#[serde(skip_serializing)]
	pub is_deleted: bool,
	#[sea_orm(nullable)]
	#[serde(skip_serializing)]
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,
}

impl From<Vec<Model>> for PrimaryIds {
	fn from(m: Vec<Model>) -> PrimaryIds {
		let mut ids: Vec<PrimaryId> = m.iter().map(|m| m.address_id).collect();

		ids.sort_unstable();
		ids.dedup();

		PrimaryIds(ids)
	}
}

pub use ActiveModel as AddressActiveModel;
pub use Model as Address;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "entity::Entity",
		from = "Column::EntityId",
		to = "EntityColumn::EntityId"
	)]
	Entity,
}

impl ActiveModelBehavior for ActiveModel {}

impl BasicModel for Model {
	type ActiveModel = ActiveModel;
}

#[async_trait]
impl SoftDeleteModel for Model {
	type ActiveModel = ActiveModel;

	async fn get_all_deleted<C>(c: &C) -> Result<Vec<Self>>
	where
		C: ConnectionTrait,
	{
		Ok(Entity::find()
			.filter(Column::IsDeleted.eq(true))
			.filter(
				Condition::any().add(
					Column::EntityId.in_subquery(
						Query::select()
							.column(EntityColumn::EntityId)
							.from(entity::Entity)
							.and_where(EntityColumn::IsDeleted.eq(true))
							.to_owned(),
					),
				),
			)
			.all(c)
			.await?)
	}
}

impl Model {
	pub fn new_model(
		id: Option<String>,
		entity_id: PrimaryId,
		network_id: PrimaryId,
		network: &str,
		address: &str,
		description: &str,
	) -> ActiveModel {
		ActiveModel {
			entity_id: Set(entity_id),
			network_id: Set(network_id),
			network: Set(network.to_string()),
			id: Set(id.unwrap_or(utils::new_unique_id(IdPrefix::Address))),
			address: Set(address.to_string()),
			description: Set(description.to_string()),
			is_deleted: Set(false),
			..Default::default()
		}
	}

	pub async fn create_many<C>(c: &C, data: Vec<ActiveModel>) -> Result<PrimaryId>
	where
		C: ConnectionTrait,
	{
		let insert_result = Entity::insert_many(data)
			.on_conflict(
				OnConflict::columns([Column::NetworkId, Column::Address]).do_nothing().to_owned(),
			)
			.exec(c)
			.await?;

		Ok(insert_result.last_insert_id)
	}

	pub async fn get_all_by_addresses<C>(
		c: &C,
		mut addresses: Vec<String>,
		is_deleted: Option<bool>,
	) -> Result<Vec<Self>>
	where
		C: ConnectionTrait,
	{
		addresses.sort_unstable();
		addresses.dedup();

		let mut q = Entity::find().filter(Column::Address.is_in(addresses));

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Column::IsDeleted.eq(is_deleted))
		}

		Ok(q.all(c).await?)
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

	pub async fn get_all_by_network_ids<C>(
		c: &C,
		network_ids: PrimaryIds,
		is_deleted: Option<bool>,
	) -> Result<Vec<Self>>
	where
		C: ConnectionTrait,
	{
		let mut q = Entity::find().filter(Column::NetworkId.is_in(network_ids));

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Column::IsDeleted.eq(is_deleted))
		}

		Ok(q.all(c).await?)
	}

	pub async fn get_all_by_network_id_and_addresses<C>(
		c: &C,
		network_id: PrimaryId,
		mut addresses: Vec<String>,
		is_deleted: Option<bool>,
	) -> Result<Vec<Self>>
	where
		C: ConnectionTrait,
	{
		addresses.sort_unstable();
		addresses.dedup();

		let mut q = Entity::find()
			.filter(Column::NetworkId.eq(network_id))
			.filter(Column::Address.is_in(addresses));

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Column::IsDeleted.eq(is_deleted))
		}

		Ok(q.all(c).await?)
	}
}
