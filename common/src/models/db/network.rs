use eyre::Result;
use sea_orm::{
	entity::prelude::*,
	sea_query::{func::Func, Expr},
	Condition, ConnectionTrait, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
	models::{BasicModel, PrimaryId, PrimaryIds, SoftDeleteModel},
	utils, Blockchain, Env, IdPrefix,
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
	pub env: Env,
	pub blockchain: Blockchain,
	pub chain_id: i64,
	pub block_time_ms: i64,
	pub rpc_endpoints: Json,
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
		let mut ids: Vec<PrimaryId> = m.iter().map(|m| m.network_id).collect();

		ids.sort_unstable();
		ids.dedup();

		PrimaryIds(ids)
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanitizedNetwork {
	pub id: String,
	pub name: String,
	pub env: Env,
	pub chain_id: i64,
}

impl From<Model> for SanitizedNetwork {
	fn from(m: Model) -> SanitizedNetwork {
		SanitizedNetwork { id: m.id, name: m.name, env: m.env, chain_id: m.chain_id }
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
		name: &str,
		env: Env,
		blockchain: Blockchain,
		chain_id: i64,
		block_time_ms: i64,
		rpc_endpoints: Vec<String>,
		rps: i32,
	) -> ActiveModel {
		ActiveModel {
			id: Set(utils::new_unique_id(IdPrefix::Network)),
			name: Set(name.to_string()),
			env: Set(env),
			blockchain: Set(blockchain),
			chain_id: Set(chain_id),
			block_time_ms: Set(block_time_ms),
			rpc_endpoints: Set(json!(rpc_endpoints)),
			is_deleted: Set(false),
			rps: Set(rps),
			..Default::default()
		}
	}

	pub async fn get_all_by_env<C>(c: &C, env: Env, is_deleted: Option<bool>) -> Result<Vec<Self>>
	where
		C: ConnectionTrait,
	{
		let mut q = Entity::find().filter(Column::Env.eq(env));

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

	pub async fn get_by_env_blockchain_and_chain_id<C>(
		c: &C,
		env: Env,
		blockchain: Blockchain,
		chain_id: i64,
		is_deleted: Option<bool>,
	) -> Result<Option<Self>>
	where
		C: ConnectionTrait,
	{
		let mut q = Entity::find()
			.filter(Column::Env.eq(env))
			.filter(Column::Blockchain.eq(blockchain))
			.filter(Column::ChainId.eq(chain_id));

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Column::IsDeleted.eq(is_deleted))
		}

		Ok(q.one(c).await?)
	}
}
