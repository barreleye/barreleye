use async_trait::async_trait;
pub use sea_orm_migration::prelude::*;

mod m20230101_000001_create_configs;
mod m20230101_000002_create_networks;
mod m20230101_000004_create_api_keys;
mod m20230101_000005_create_labels;
mod m20230101_000006_create_labeled_addresses;

pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![
			Box::new(m20230101_000001_create_configs::Migration),
			Box::new(m20230101_000002_create_networks::Migration),
			Box::new(m20230101_000004_create_api_keys::Migration),
			Box::new(m20230101_000005_create_labels::Migration),
			Box::new(m20230101_000006_create_labeled_addresses::Migration),
		]
	}
}
