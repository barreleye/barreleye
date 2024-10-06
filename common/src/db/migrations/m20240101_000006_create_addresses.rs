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
					.table(Addresses::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Addresses::AddressId)
							.big_integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(Addresses::EntityId).big_integer().not_null())
					.col(ColumnDef::new(Addresses::NetworkId).big_integer().not_null())
					.col(ColumnDef::new(Addresses::Network).string().not_null())
					.col(ColumnDef::new(Addresses::Id).unique_key().string().not_null())
					.col(ColumnDef::new(Addresses::Address).string().not_null())
					.col(ColumnDef::new(Addresses::Description).string().not_null())
					.col(ColumnDef::new(Addresses::Data).json().not_null())
					.col(ColumnDef::new(Addresses::IsDeleted).boolean().not_null())
					.col(ColumnDef::new(Addresses::UpdatedAt).date_time().null())
					.col(
						ColumnDef::new(Addresses::CreatedAt)
							.date_time()
							.not_null()
							.extra("DEFAULT CURRENT_TIMESTAMP".to_owned()),
					)
					.foreign_key(
						&mut sea_query::ForeignKey::create()
							.name("fk_addresses_entity_id")
							.from(Addresses::Table, Addresses::EntityId)
							.to(Alias::new("entities"), Alias::new("entity_id"))
							.on_delete(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.foreign_key(
						&mut sea_query::ForeignKey::create()
							.name("fk_addresses_network_id")
							.from(Addresses::Table, Addresses::NetworkId)
							.to(Alias::new("networks"), Alias::new("network_id"))
							.on_delete(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.foreign_key(
						&mut sea_query::ForeignKey::create()
							.name("fk_addresses_network")
							.from(Addresses::Table, Addresses::Network)
							.to(Alias::new("networks"), Alias::new("id"))
							.on_update(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("ux_addresses_entity_id_network_id_address")
					.table(Addresses::Table)
					.unique()
					.col(Addresses::EntityId)
					.col(Addresses::NetworkId)
					.col(Addresses::Address)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("ix_addresses_is_deleted")
					.table(Addresses::Table)
					.col(Addresses::IsDeleted)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Addresses::Table).to_owned()).await
	}
}

#[derive(Iden)]
enum Addresses {
	#[iden = "addresses"]
	Table,
	AddressId,
	EntityId,
	NetworkId,
	Network,
	Id,
	Address,
	Description,
	Data,
	IsDeleted,
	UpdatedAt,
	CreatedAt,
}
