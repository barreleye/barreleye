use async_trait::async_trait;
use sea_orm::EntityTrait;
use sea_orm_migration::prelude::*;

use crate::{models::account, utils, IdPrefix};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(ApiKeys::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(ApiKeys::ApiKeyId)
							.big_integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(
						ColumnDef::new(ApiKeys::AccountId)
							.big_integer()
							.not_null(),
					)
					.col(
						ColumnDef::new(ApiKeys::Id)
							.unique_key()
							.string()
							.not_null(),
					)
					.col(
						ColumnDef::new(ApiKeys::Uuid)
							.unique_key()
							.uuid()
							.not_null(),
					)
					.col(ColumnDef::new(ApiKeys::UpdatedAt).date_time().null())
					.col(
						ColumnDef::new(ApiKeys::CreatedAt)
							.date_time()
							.not_null()
							.extra("DEFAULT CURRENT_TIMESTAMP".to_owned()),
					)
					.foreign_key(
						&mut sea_query::ForeignKey::create()
							.name("fk_api_keys_account_id")
							.from(ApiKeys::Table, ApiKeys::AccountId)
							.to(
								Alias::new("accounts"),
								Alias::new("account_id"),
							)
							.to_owned(),
					)
					.to_owned(),
			)
			.await?;

		let account_id = account::Entity::find()
			.one(manager.get_connection())
			.await?
			.unwrap()
			.account_id;

		manager
			.exec_stmt(
				Query::insert()
					.into_table(ApiKeys::Table)
					.columns([ApiKeys::AccountId, ApiKeys::Id, ApiKeys::Uuid])
					.values_panic([
						account_id.into(),
						utils::unique_id(IdPrefix::ApiKey, "default").into(),
						utils::new_uuid().into(),
					])
					.on_conflict(
						OnConflict::columns([ApiKeys::Id])
							.do_nothing()
							.to_owned(),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(ApiKeys::Table).to_owned()).await
	}
}

#[derive(Iden)]
enum ApiKeys {
	#[iden = "api_keys"]
	Table,
	ApiKeyId,
	AccountId,
	Id,
	Uuid,
	UpdatedAt,
	CreatedAt,
}
