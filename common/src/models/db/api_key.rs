use base58::ToBase58;
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

#[derive(
	Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel,
)]
#[sea_orm(table_name = "api_keys")]
#[serde(rename_all = "camelCase")]
pub struct Model {
	#[sea_orm(primary_key)]
	#[serde(skip_serializing, skip_deserializing)]
	pub api_key_id: PrimaryId,
	pub id: String,
	#[serde(skip_serializing, skip_deserializing)]
	pub secret_key: Option<String>,
	#[serde(skip_serializing, skip_deserializing)]
	pub secret_key_hash: Vec<u8>,
	pub is_active: bool,
	#[sea_orm(nullable)]
	#[serde(skip_serializing)]
	pub updated_at: Option<DateTime>,
	pub created_at: DateTime,

	#[sea_orm(ignore)]
	pub key: Option<String>,
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
	pub fn new_model(id: Option<String>) -> ActiveModel {
		let (secret_key, secret_key_hash) = Self::generate_key();

		ActiveModel {
			id: Set(id.unwrap_or(utils::new_unique_id(IdPrefix::ApiKey))),
			secret_key: Set(Some(format!("sk_{secret_key}"))),
			secret_key_hash: Set(secret_key_hash),
			is_active: Set(true),
			..Default::default()
		}
	}

	pub async fn get_by_hashing<C>(
		c: &C,
		secret_key: &str,
	) -> Result<Option<Self>>
	where
		C: ConnectionTrait,
	{
		let secret_key_postfix = {
			let tmp: Vec<&str> = secret_key.split('_').collect();
			tmp[tmp.len() - 1]
		};

		let secret_key_hash = utils::sha256(secret_key_postfix);

		Ok(Entity::find()
			.filter(Column::SecretKeyHash.eq(secret_key_hash))
			.one(c)
			.await?)
	}

	pub fn format(&self) -> Self {
		let mut key = None;
		if let Some(secret_key) = self.secret_key.clone() {
			key = Some(secret_key);
		}

		Self { key, ..self.clone() }
	}

	pub fn generate_key() -> (String, Vec<u8>) {
		let input = utils::new_uuid().to_string();

		let hash = utils::sha256(&input);
		let secret_key = hash.to_base58();
		let secret_key_hash = utils::sha256(&secret_key);

		(format!("sk_{secret_key}"), secret_key_hash)
	}

	pub async fn hide_key<C>(c: &C, api_key_id: PrimaryId) -> Result<()>
	where
		C: ConnectionTrait,
	{
		Entity::update(ActiveModel {
			api_key_id: Set(api_key_id),
			secret_key: Set(None),
			updated_at: Set(Some(utils::now())),
			..Default::default()
		})
		.exec(c)
		.await?;

		Ok(())
	}
}
