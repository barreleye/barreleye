use axum::{
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use derive_more::{Display, Error};
use eyre::{ErrReport, Report};
use sea_orm::DbErr;
use serde_json::json;

#[derive(Debug, Display, Error)]
pub enum ServerError {
	#[display(fmt = "unauthorized")]
	Unauthorized,

	#[display(fmt = "validation error @ `{field}`")]
	Validation { field: String },

	#[display(fmt = "invalid parameter @ `{field}`: {value}")]
	InvalidParam { field: String, value: String },

	#[display(fmt = "invalid value(s) @ parameter `{field}`: {values}")]
	InvalidValues { field: String, values: String },

	#[display(fmt = "exceeded limit @ parameter `{field}`: {limit}")]
	ExceededLimit { field: String, limit: usize },

	#[display(fmt = "missing input params")]
	MissingInputParams,

	#[display(fmt = "could not connect to `{name}`")]
	InvalidService { name: String },

	#[display(fmt = "duplicate found @ `{field}`: {value}")]
	Duplicate { field: String, value: String },

	#[display(fmt = "duplicates found @ `{field}`: {values}")]
	Duplicates { field: String, values: String },

	#[display(fmt = "bad request: {reason}")]
	BadRequest { reason: String },

	#[display(fmt = "conflict: {reason}")]
	Conflict { reason: String },

	#[display(fmt = "not found")]
	NotFound,

	#[display(fmt = "rekt")]
	Internal { error: Report },
}

impl IntoResponse for ServerError {
	fn into_response(self) -> Response {
		let http_code = match self {
			ServerError::NotFound => StatusCode::NOT_FOUND,
			ServerError::Unauthorized => StatusCode::UNAUTHORIZED,
			ServerError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
			_ => StatusCode::BAD_REQUEST,
		};

		let body = Json(json!({
			"error": self.to_string(),
		}));

		(http_code, body).into_response()
	}
}

impl From<DbErr> for ServerError {
	fn from(e: DbErr) -> ServerError {
		ServerError::Internal { error: Report::new(e) }
	}
}

impl From<ErrReport> for ServerError {
	fn from(e: ErrReport) -> ServerError {
		ServerError::Internal { error: e }
	}
}

impl From<serde_json::Error> for ServerError {
	fn from(e: serde_json::Error) -> ServerError {
		ServerError::Internal { error: Report::new(e) }
	}
}
