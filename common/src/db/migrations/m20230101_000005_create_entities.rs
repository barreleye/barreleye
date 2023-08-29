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
					.table(Entities::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Entities::EntityId)
							.big_integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(Entities::Id).unique_key().string().not_null())
					.col(ColumnDef::new(Entities::Name).unique_key().string().null())
					.col(ColumnDef::new(Entities::Description).string().not_null())
					.col(ColumnDef::new(Entities::Data).json().not_null())
					.col(ColumnDef::new(Entities::IsDeleted).boolean().not_null())
					.col(ColumnDef::new(Entities::UpdatedAt).date_time().null())
					.col(
						ColumnDef::new(Entities::CreatedAt)
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
					.name("ix_entities_is_deleted")
					.table(Entities::Table)
					.col(Entities::IsDeleted)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Entities::Table).to_owned()).await
	}
}

#[derive(Iden)]
enum Entities {
	#[iden = "entities"]
	Table,
	EntityId,
	Id,
	Name,
	Description,
	Data,
	IsDeleted,
	UpdatedAt,
	CreatedAt,
}
