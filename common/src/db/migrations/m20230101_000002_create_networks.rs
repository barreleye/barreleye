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
					.col(ColumnDef::new(Networks::Tag).string().not_null())
					.col(ColumnDef::new(Networks::Env).small_integer().not_null())
					.col(ColumnDef::new(Networks::Blockchain).small_integer().not_null())
					.col(ColumnDef::new(Networks::ChainId).big_integer().not_null())
					.col(ColumnDef::new(Networks::BlockTimeMs).big_integer().not_null())
					.col(ColumnDef::new(Networks::RpcEndpoints).json().not_null())
					.col(ColumnDef::new(Networks::Rps).integer().not_null())
					.col(ColumnDef::new(Networks::IsActive).boolean().not_null())
					.col(ColumnDef::new(Networks::UpdatedAt).date_time().null())
					.col(
						ColumnDef::new(Networks::CreatedAt)
							.date_time()
							.not_null()
							.extra("DEFAULT CURRENT_TIMESTAMP".to_owned()),
					)
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
	Tag,
	Env,
	Blockchain,
	ChainId,
	BlockTimeMs,
	RpcEndpoints,
	Rps,
	IsActive,
	UpdatedAt,
	CreatedAt,
}
