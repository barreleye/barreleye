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
					.table(EntityTags::Table)
					.if_not_exists()
					.col(ColumnDef::new(EntityTags::EntityId).big_integer().not_null())
					.col(ColumnDef::new(EntityTags::TagId).big_integer().not_null())
					.col(
						ColumnDef::new(EntityTags::CreatedAt)
							.date_time()
							.not_null()
							.extra("DEFAULT CURRENT_TIMESTAMP".to_owned()),
					)
					.primary_key(
						sea_query::Index::create().col(EntityTags::EntityId).col(EntityTags::TagId),
					)
					.foreign_key(
						&mut sea_query::ForeignKey::create()
							.name("fk_entity_tags_entity_id")
							.from(EntityTags::Table, EntityTags::EntityId)
							.to(Alias::new("entities"), Alias::new("entity_id"))
							.on_delete(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.foreign_key(
						&mut sea_query::ForeignKey::create()
							.name("fk_entity_tags_tag_id")
							.from(EntityTags::Table, EntityTags::TagId)
							.to(Alias::new("tags"), Alias::new("tag_id"))
							.on_delete(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(EntityTags::Table).to_owned()).await
	}
}

#[derive(Iden)]
enum EntityTags {
	#[iden = "entity_tags"]
	Table,
	EntityId,
	TagId,
	CreatedAt,
}
