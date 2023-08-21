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
					.table(Networks::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Networks::NetworkId)
							.big_integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(Networks::Id).unique_key().string().not_null())
					.col(ColumnDef::new(Networks::Name).unique_key().string().not_null())
					.col(ColumnDef::new(Networks::Architecture).small_integer().not_null())
					.col(ColumnDef::new(Networks::ChainId).big_integer().not_null())
					.col(ColumnDef::new(Networks::BlockTime).big_integer().not_null())
					.col(ColumnDef::new(Networks::RpcEndpoint).string().not_null())
					.col(ColumnDef::new(Networks::Rps).integer().not_null())
					.col(ColumnDef::new(Networks::IsDeleted).boolean().not_null())
					.col(ColumnDef::new(Networks::UpdatedAt).date_time().null())
					.col(
						ColumnDef::new(Networks::CreatedAt)
							.date_time()
							.not_null()
							.extra("DEFAULT CURRENT_TIMESTAMP".to_owned()),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("ix_networks_is_deleted")
					.table(Networks::Table)
					.col(Networks::IsDeleted)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Networks::Table).to_owned()).await
	}
}

#[derive(Iden)]
enum Networks {
	#[iden = "networks"]
	Table,
	NetworkId,
	Id,
	Name,
	Architecture,
	ChainId,
	BlockTime,
	RpcEndpoint,
	Rps,
	IsDeleted,
	UpdatedAt,
	CreatedAt,
}
