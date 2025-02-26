use async_trait::async_trait;
use console::style;
use derive_more::Display;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::{
	warehouse::{clickhouse::ClickHouse, duckdb::DuckDB},
	AppError, Settings,
};

pub mod clickhouse;
pub mod duckdb;

#[derive(Display, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum Driver {
	#[default]
	#[serde(rename = "duckdb")]
	#[display("DuckDB")]
	DuckDB,
	#[serde(rename = "clickhouse")]
	#[display("ClickHouse")]
	ClickHouse,
}

#[async_trait]
pub trait DriverTrait: Send + Sync {
	async fn new(settings: Arc<Settings>) -> Result<Self>
	where
		Self: Sized;
	async fn run_migrations(&self) -> Result<()>;
	async fn insert(&self, table: &str, serialized_data: &[String]) -> Result<()>;
	async fn select(&self, query: &str) -> Result<Vec<String>>;
	async fn delete(&self, query: &str) -> Result<()>;
}

pub struct Warehouse {
	driver: Box<dyn DriverTrait>,
}

impl Warehouse {
	pub async fn new(settings: Arc<Settings>) -> Result<Self, AppError<'static>> {
		let driver: Box<dyn DriverTrait> = match settings.warehouse_driver {
			Driver::DuckDB => Box::new(
				DuckDB::new(settings.clone())
					.await
					.map_err(|_| AppError::Warehouse { error: "could not connect".into() })?,
			),
			Driver::ClickHouse => Box::new(
				ClickHouse::new(settings.clone())
					.await
					.map_err(|_| AppError::Warehouse { error: "could not connect".into() })?,
			),
		};

		let log_message = settings
			.warehouse_path
			.as_ref()
			.map(|warehouse_path| warehouse_path.display().to_string())
			.or_else(|| {
				settings
					.warehouse_url
					.as_ref()
					.map(|warehouse_url| warehouse_url.as_str().to_string())
			});

		if let Some(url) = log_message {
			info!("{} is connected to {}", settings.warehouse_driver, style(url).bold());
		}

		Ok(Self { driver })
	}

	pub async fn run_migrations(&self) -> Result<()> {
		self.driver.run_migrations().await
	}

	pub async fn insert<T: Serialize>(&self, table: &str, data: &[T]) -> Result<()> {
		let serialized_data: Vec<String> = data
			.iter()
			.map(|item| serde_json::to_string(item))
			.collect::<Result<Vec<_>, _>>()
			.map_err(|e| eyre!(e))?;

		self.driver.insert(table, &serialized_data).await
	}

	pub async fn select<T: for<'de> Deserialize<'de>>(&self, query: &str) -> Result<Vec<T>> {
		let serialized_rows = self.driver.select(query).await?;
		let deserialized_rows: Vec<T> = serialized_rows
			.iter()
			.map(|row| serde_json::from_str(row))
			.collect::<Result<Vec<_>, _>>()
			.map_err(|e| eyre!(e))?;

		Ok(deserialized_rows)
	}

	pub async fn delete(&self, query: &str) -> Result<()> {
		self.driver.delete(query).await
	}
}
