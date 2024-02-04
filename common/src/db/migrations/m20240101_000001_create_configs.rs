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
					.table(Configs::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Configs::ConfigId)
							.big_integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(Configs::Key).unique_key().string().not_null())
					.col(ColumnDef::new(Configs::Value).string().null())
					.col(
						ColumnDef::new(Configs::UpdatedAt)
							.date_time()
							.not_null()
							.extra("DEFAULT CURRENT_TIMESTAMP".to_owned()),
					)
					.col(
						ColumnDef::new(Configs::CreatedAt)
							.date_time()
							.not_null()
							.extra("DEFAULT CURRENT_TIMESTAMP".to_owned()),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Configs::Table).to_owned()).await
	}
}

#[derive(Iden)]
enum Configs {
	#[iden = "configs"]
	Table,
	ConfigId,
	Key,
	Value,
	UpdatedAt,
	CreatedAt,
}
