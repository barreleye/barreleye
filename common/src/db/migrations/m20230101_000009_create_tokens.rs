use async_trait::async_trait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(Tokens::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Tokens::TokenId)
							.big_integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(Tokens::NetworkId).big_integer().not_null())
					.col(ColumnDef::new(Tokens::Id).unique_key().string().not_null())
					.col(ColumnDef::new(Tokens::Name).unique_key().string().not_null())
					.col(ColumnDef::new(Tokens::Symbol).unique_key().string().not_null())
					.col(ColumnDef::new(Tokens::Address).string().not_null())
					.col(ColumnDef::new(Tokens::Decimals).small_integer().not_null())
					.col(ColumnDef::new(Tokens::UpdatedAt).date_time().null())
					.col(
						ColumnDef::new(Tokens::CreatedAt)
							.date_time()
							.not_null()
							.extra("DEFAULT CURRENT_TIMESTAMP".to_owned()),
					)
					.foreign_key(
						&mut sea_query::ForeignKey::create()
							.name("fk_tokens_network_id")
							.from(Tokens::Table, Tokens::NetworkId)
							.to(Alias::new("networks"), Alias::new("network_id"))
							.on_delete(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("ux_tokens_network_id_address")
					.table(Tokens::Table)
					.unique()
					.col(Tokens::NetworkId)
					.col(Tokens::Address)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("ix_tokens_network_id")
					.table(Tokens::Table)
					.col(Tokens::NetworkId)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Tokens::Table).to_owned()).await
	}
}

#[derive(Iden)]
enum Tokens {
	#[iden = "tokens"]
	Table,
	TokenId,
	NetworkId,
	Id,
	Name,
	Symbol,
	Address,
	Decimals,
	UpdatedAt,
	CreatedAt,
}
