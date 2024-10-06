use async_trait::async_trait;
use eyre::Result;
use sea_orm::{
	entity::prelude::*,
	sea_query::{types::*, Expr},
	ActiveValue, QuerySelect,
};
use sea_orm_migration::prelude::IntoCondition;
use std::ops::{Deref, DerefMut};

use crate::{utils, IdPrefix};
pub use db::*;
pub use warehouse::*;

mod db;
mod warehouse;

pub type PrimaryId = i64;

#[derive(Clone)]
pub struct PrimaryIds(Vec<PrimaryId>);

impl From<Vec<PrimaryId>> for PrimaryIds {
	fn from(v: Vec<PrimaryId>) -> PrimaryIds {
		PrimaryIds(v)
	}
}

impl From<PrimaryId> for PrimaryIds {
	fn from(p: PrimaryId) -> PrimaryIds {
		PrimaryIds(vec![p])
	}
}

impl IntoIterator for PrimaryIds {
	type Item = PrimaryId;
	type IntoIter = <Vec<PrimaryId> as IntoIterator>::IntoIter;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl Deref for PrimaryIds {
	type Target = [PrimaryId];

	fn deref(&self) -> &Self::Target {
		&self.0[..]
	}
}

impl DerefMut for PrimaryIds {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0[..]
	}
}

pub fn set<T>(v: T) -> ActiveValue<T>
where
	T: Into<sea_orm::Value>,
{
	ActiveValue::set(v)
}

pub fn optional_set<T>(o: Option<T>) -> ActiveValue<T>
where
	T: Into<sea_orm::Value>,
{
	match o {
		Some(v) => ActiveValue::set(v),
		_ => ActiveValue::not_set(),
	}
}

pub fn is_valid_id(id: &str, id_prefix: IdPrefix) -> bool {
	// check prefix
	if !id.starts_with(&format!("{}_", id_prefix)) {
		return false;
	}

	// check min & max lengths
	let contents = &id[id_prefix.to_string().len() + 1..];
	if contents.is_empty() || contents.len() > 32 {
		return false;
	}

	// check alphanumeric value
	if !contents.chars().all(char::is_alphanumeric) {
		return false;
	}

	true
}

#[async_trait]
pub trait BasicModel {
	type ActiveModel: ActiveModelTrait + ActiveModelBehavior + Sized + Send;

    async fn create<C>(
		c: &C,
		data: Self::ActiveModel,
	) -> Result<<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::PrimaryKey as
	PrimaryKeyTrait>::ValueType>
	where
		C: ConnectionTrait,
	{
		let insert_result =
			<Self::ActiveModel as ActiveModelTrait>::Entity::insert(data).exec(c).await?;

		Ok(insert_result.last_insert_id)
	}

    async fn create_many<C>(
		c: &C,
		data: Vec<Self::ActiveModel>,
	) -> Result<<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::PrimaryKey as
	PrimaryKeyTrait>::ValueType>
	where
		C: ConnectionTrait,
	{
		let insert_result =
			<Self::ActiveModel as ActiveModelTrait>::Entity::insert_many(data).exec(c).await?;

		Ok(insert_result.last_insert_id)
	}

	async fn get<C>(
		c: &C,
		primary_id: <<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::PrimaryKey as
		PrimaryKeyTrait>::ValueType,
	) -> Result<Option<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::Model>>
	where
		C: ConnectionTrait,
	{
		Ok(<Self::ActiveModel as ActiveModelTrait>::Entity::find_by_id(primary_id).one(c).await?)
	}

	async fn get_by_id<C>(
		c: &C,
		id: &str,
	) -> Result<Option<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::Model>>
	where
		C: ConnectionTrait,
	{
		Ok(<Self::ActiveModel as ActiveModelTrait>::Entity::find()
			.filter(Expr::col(Alias::new("id")).eq(id))
			.one(c)
			.await?)
	}

	async fn get_all<C>(
		c: &C,
	) -> Result<Vec<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::Model>>
	where
		C: ConnectionTrait,
	{
		Ok(<Self::ActiveModel as ActiveModelTrait>::Entity::find().all(c).await?)
	}

	async fn get_all_paginated<C>(
		c: &C,
		offset: Option<u64>,
		limit: Option<u64>,
	) -> Result<Vec<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::Model>>
	where
		C: ConnectionTrait,
	{
		let mut q = <Self::ActiveModel as ActiveModelTrait>::Entity::find();

		if let Some(v) = offset {
			q = q.offset(v);
		}
		if let Some(v) = limit {
			q = q.limit(v);
		}

		Ok(q.all(c).await?)
	}

	async fn get_all_where<C, F>(
		c: &C,
		filter: F,
	) -> Result<Vec<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::Model>>
	where
		C: ConnectionTrait,
		F: IntoCondition + Send,
	{
		Ok(<Self::ActiveModel as ActiveModelTrait>::Entity::find().filter(filter).all(c).await?)
	}

	async fn get_all_paginated_where<C, F>(
		c: &C,
		filter: F,
		offset: Option<u64>,
		limit: Option<u64>,
	) -> Result<Vec<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::Model>>
	where
		C: ConnectionTrait,
		F: IntoCondition + Send,
	{
		let mut q = <Self::ActiveModel as ActiveModelTrait>::Entity::find().filter(filter);

		if let Some(v) = offset {
			q = q.offset(v);
		}
		if let Some(v) = limit {
			q = q.limit(v);
		}

		Ok(q.all(c).await?)
	}

	async fn update_by_id<C>(c: &C, id: &str, data: Self::ActiveModel) -> Result<bool>
	where
		C: ConnectionTrait,
	{
		let res = <Self::ActiveModel as ActiveModelTrait>::Entity::update_many()
			.col_expr(Alias::new("updated_at"), Expr::value(utils::now()))
			.set(data)
			.filter(Expr::col(Alias::new("id")).eq(id))
			.exec(c)
			.await?;

		Ok(res.rows_affected == 1)
	}

	async fn update_all_where<C, F>(c: &C, filter: F, data: Self::ActiveModel) -> Result<u64>
	where
		C: ConnectionTrait,
		F: IntoCondition + Send,
	{
		let res = <Self::ActiveModel as ActiveModelTrait>::Entity::update_many()
			.col_expr(Alias::new("updated_at"), Expr::value(utils::now()))
			.set(data)
			.filter(filter)
			.exec(c)
			.await?;

		Ok(res.rows_affected)
	}

	async fn delete<C>(
		c: &C,
		primary_id: <<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::PrimaryKey as
		PrimaryKeyTrait>::ValueType,
	) -> Result<bool>
	where
		C: ConnectionTrait,
	{
		let res = <Self::ActiveModel as ActiveModelTrait>::Entity::delete_by_id(primary_id)
			.exec(c)
			.await?;

		Ok(res.rows_affected == 1)
	}

	async fn delete_by_id<C>(c: &C, id: &str) -> Result<bool>
	where
		C: ConnectionTrait,
	{
		let res = <Self::ActiveModel as ActiveModelTrait>::Entity::delete_many()
			.filter(Expr::col(Alias::new("id")).eq(id))
			.exec(c)
			.await?;

		Ok(res.rows_affected == 1)
	}

	async fn delete_all_where<C, F>(c: &C, filter: F) -> Result<u64>
	where
		C: ConnectionTrait,
		F: IntoCondition + Send,
	{
		let res = <Self::ActiveModel as ActiveModelTrait>::Entity::delete_many()
			.filter(filter)
			.exec(c)
			.await?;

		Ok(res.rows_affected)
	}
}

#[async_trait]
pub trait SoftDeleteModel {
	type ActiveModel: ActiveModelTrait + ActiveModelBehavior + Sized + Send;

	async fn get_existing_by_id<C>(
		c: &C,
		id: &str,
	) -> Result<Option<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::Model>>
	where
		C: ConnectionTrait,
	{
		Ok(<Self::ActiveModel as ActiveModelTrait>::Entity::find()
			.filter(Expr::col(Alias::new("id")).eq(id))
			.filter(Expr::col(Alias::new("is_deleted")).eq(false))
			.one(c)
			.await?)
	}

	async fn get_all_existing<C>(
		c: &C,
		is_deleted: Option<bool>,
	) -> Result<Vec<<<Self::ActiveModel as ActiveModelTrait>::Entity as EntityTrait>::Model>>
	where
		C: ConnectionTrait,
	{
		let mut q = <Self::ActiveModel as ActiveModelTrait>::Entity::find();

		if let Some(is_deleted) = is_deleted {
			q = q.filter(Expr::col(Alias::new("is_deleted")).eq(is_deleted))
		}

		Ok(q.all(c).await?)
	}

	async fn prune_all<C>(c: &C) -> Result<u64>
	where
		C: ConnectionTrait,
	{
		let res = <Self::ActiveModel as ActiveModelTrait>::Entity::delete_many()
			.filter(Expr::col(Alias::new("is_deleted")).eq(true))
			.exec(c)
			.await?;

		Ok(res.rows_affected)
	}

	async fn prune_all_where<C, F>(c: &C, filter: F) -> Result<u64>
	where
		C: ConnectionTrait,
		F: IntoCondition + Send,
	{
		let res = <Self::ActiveModel as ActiveModelTrait>::Entity::delete_many()
			.filter(Expr::col(Alias::new("is_deleted")).eq(true))
			.filter(filter)
			.exec(c)
			.await?;

		Ok(res.rows_affected)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	#[test]
	fn test_is_valid_id() {
		let data = HashMap::from([
			(("net_abc", IdPrefix::Network), true),
			(("net_a1b2c3", IdPrefix::Network), true),
			(("net_a", IdPrefix::Network), true),
			(("net_1", IdPrefix::Network), true),
			(("net_", IdPrefix::Network), false),
			(("net_?", IdPrefix::Network), false),
			(("net_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", IdPrefix::Network), true),
			(("net_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaA", IdPrefix::Network), false),
		]);

		for (input, output) in data.into_iter() {
			assert_eq!(is_valid_id(&input.0, input.1), output)
		}
	}
}
