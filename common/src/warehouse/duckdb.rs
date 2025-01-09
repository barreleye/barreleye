use async_trait::async_trait;
use duckdb::{Connection, ToSql};
use eyre::{eyre, Result};
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use tokio::task::spawn_blocking;

use super::DriverTrait;
use crate::Settings;

pub struct DuckDB {
	connection: Arc<Mutex<duckdb::Connection>>,
}

fn json_value_to_sql(value: &JsonValue) -> Box<dyn ToSql> {
	match value {
		JsonValue::Null => Box::new(None::<String>),
		JsonValue::Bool(b) => Box::new(*b),
		JsonValue::Number(n) => {
			if n.is_i64() {
				Box::new(n.as_i64().unwrap())
			} else if n.is_u64() {
				Box::new(n.as_u64().unwrap())
			} else {
				Box::new(n.as_f64().unwrap())
			}
		}
		JsonValue::String(s) => Box::new(s.clone()),
		JsonValue::Array(_) | JsonValue::Object(_) => Box::new(value.to_string()),
	}
}

#[async_trait]
impl DriverTrait for DuckDB {
	async fn new(settings: Arc<Settings>) -> Result<Self> {
		let connection = spawn_blocking(move || {
			Connection::open(&settings.warehouse)
				.map_err(|e| eyre!("Failed to open DuckDB connection: {}", e))
		})
		.await??;

		Ok(Self { connection: Arc::new(Mutex::new(connection)) })
	}

	async fn run_migrations(&self) -> Result<()> {
		Ok(())
	}

	async fn insert(&self, table: &str, serialized_data: &[String]) -> Result<()> {
		let table = table.to_string();
		let data = serialized_data.to_vec();
		let conn = self.connection.clone();

		spawn_blocking(move || -> Result<()> {
			let mut conn = conn.lock().map_err(|e| eyre!("Failed to acquire lock: {}", e))?;
			let tx = conn.transaction()?;

			for json_str in data {
				let value: JsonValue = serde_json::from_str(&json_str)?;
				let columns: Vec<&str> =
					value.as_object().unwrap().keys().map(|s| s.as_str()).collect();
				let placeholders: Vec<String> =
					(1..=columns.len()).map(|i| format!("${}", i)).collect();

				let query = format!(
					"INSERT INTO {} ({}) VALUES ({})",
					table,
					columns.join(", "),
					placeholders.join(", ")
				);

				let params: Vec<Box<dyn ToSql>> =
					value.as_object().unwrap().values().map(json_value_to_sql).collect();

				tx.execute(
					&query,
					params.iter().map(|p| p.as_ref()).collect::<Vec<&dyn ToSql>>().as_slice(),
				)?;
			}

			tx.commit()?;
			Ok(())
		})
		.await?
	}

	async fn select(&self, query: &str) -> Result<Vec<String>> {
		let connection = self.connection.lock().map_err(|e| eyre::eyre!("Lock poisoned: {}", e))?;

		let mut statement = connection.prepare(query)?;

		let column_count = statement.column_count();
		let column_names = statement.column_names();

		let mut rows = statement.query([])?;
		let mut results = Vec::new();

		while let Some(row) = rows.next()? {
			let mut json_map = serde_json::Map::new();

			for (i, col_name) in column_names.iter().enumerate().take(column_count) {
				let value: JsonValue = match row.get::<usize, Option<i64>>(i) {
					Ok(Some(val)) => JsonValue::from(val),
					Ok(None) => JsonValue::Null,
					Err(_) => match row.get::<usize, Option<f64>>(i) {
						Ok(Some(val)) => JsonValue::from(val),
						Ok(None) => JsonValue::Null,
						Err(_) => match row.get::<usize, Option<String>>(i) {
							Ok(Some(val)) => JsonValue::from(val),
							Ok(None) => JsonValue::Null,
							Err(_) => JsonValue::Null,
						},
					},
				};

				json_map.insert(col_name.to_string(), value);
			}

			results.push(JsonValue::Object(json_map).to_string());
		}

		Ok(results)
	}

	async fn delete(&self, query: &str) -> Result<()> {
		let query = query.to_string();
		let conn = self.connection.clone();

		spawn_blocking(move || -> Result<()> {
			let conn = conn.lock().map_err(|e| eyre!("Failed to acquire lock: {}", e))?;
			conn.execute(&query, [])?;
			Ok(())
		})
		.await?
	}
}
