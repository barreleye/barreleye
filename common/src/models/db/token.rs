use async_trait::async_trait;
use eyre::Result;
use sea_orm::{
	entity::{prelude::*, *},
	ConnectionTrait,
};
use sea_orm_migration::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::{
	models::{BasicModel, PrimaryId, PrimaryIds, SoftDeleteModel},
	utils, IdPrefix,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "tokens")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub token_id: PrimaryId,
	#[serde(skip_serializing)]
	pub network_id: PrimaryId,
	pub id: String,
	pub chain_id: i64,
	pub name: String,
	pub symbol: String,
	pub address: String,
	pub decimals: i16,
	#[serde(skip_serializing)]
	pub is_deleted: bool,
	#[sea_orm(nullable)]
	#[serde(skip_serializing)]
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,
}

impl From<Vec<Model>> for PrimaryIds {
	fn from(m: Vec<Model>) -> PrimaryIds {
		let ids: HashSet<PrimaryId> = m.iter().map(|m| m.token_id).collect();
		PrimaryIds(ids.into_iter().collect())
	}
}

pub use ActiveModel as TokenActiveModel;
pub use Model as Token;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl BasicModel for Model {
	type ActiveModel = ActiveModel;
}

#[async_trait]
impl SoftDeleteModel for Model {
	type ActiveModel = ActiveModel;
}

impl Model {
	pub fn new_model(
		id: Option<String>,
		network_id: PrimaryId,
		chain_id: i64,
		name: &str,
		symbol: &str,
		address: &str,
		decimals: i16,
	) -> ActiveModel {
		ActiveModel {
			id: Set(id.unwrap_or(utils::new_unique_id(IdPrefix::Token))),
			network_id: Set(network_id),
			chain_id: Set(chain_id),
			name: Set(name.to_string()),
			symbol: Set(symbol.to_string()),
			address: Set(address.to_string()),
			decimals: Set(decimals),
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
				OnConflict::columns([Column::NetworkId, Column::ChainId, Column::Address])
					.do_nothing()
					.to_owned(),
			)
			.exec(c)
			.await?;

		Ok(insert_result.last_insert_id)
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
}
