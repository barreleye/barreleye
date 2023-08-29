use eyre::Result;
use sea_orm::{
	entity::prelude::*,
	sea_query::{func::Func, Expr},
	Condition, ConnectionTrait, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::{
	models::{BasicModel, PrimaryId, PrimaryIds, SoftDeleteModel},
	utils, Architecture, IdPrefix,
};

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "networks")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub network_id: PrimaryId,
	pub id: String,
	pub name: String,
	pub architecture: Architecture,
	pub chain_id: i64,
	pub block_time: i64,
	pub rpc_endpoint: String,
	pub rps: i32,
	#[serde(skip_serializing)]
	pub is_deleted: bool,
	#[sea_orm(nullable)]
	#[serde(skip_serializing)]
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,
}

impl From<Vec<Model>> for PrimaryIds {
	fn from(m: Vec<Model>) -> PrimaryIds {
		let ids: HashSet<PrimaryId> = m.iter().map(|m| m.network_id).collect();
		PrimaryIds(ids.into_iter().collect())
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanitizedNetwork {
	pub id: String,
	pub name: String,
	pub chain_id: i64,
}

impl From<Model> for SanitizedNetwork {
	fn from(m: Model) -> SanitizedNetwork {
		SanitizedNetwork { id: m.id, name: m.name, chain_id: m.chain_id }
	}
}

pub use ActiveModel as NetworkActiveModel;
pub use Model as Network;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

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
		name: &str,
		architecture: Architecture,
		chain_id: i64,
		block_time: i64,
		rpc_endpoint: String,
		rps: i32,
	) -> ActiveModel {
		ActiveModel {
			id: Set(id.unwrap_or(utils::new_unique_id(IdPrefix::Network))),
			name: Set(name.to_string()),
			architecture: Set(architecture),
			chain_id: Set(chain_id),
			block_time: Set(block_time),
			rpc_endpoint: Set(rpc_endpoint),
			is_deleted: Set(false),
			rps: Set(rps),
			..Default::default()
		}
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

	pub async fn get_by_architecture_and_chain_id<C>(
		c: &C,
		architecture: Architecture,
		chain_id: i64,
		is_deleted: Option<bool>,
	) -> Result<Option<Self>>
	where
		C: ConnectionTrait,
	{
		let mut q = Entity::find()
			.filter(Column::Architecture.eq(architecture))
			.filter(Column::ChainId.eq(chain_id));

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Column::IsDeleted.eq(is_deleted))
		}

		Ok(q.one(c).await?)
	}
}
