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
	#[display("unauthorized")]
	Unauthorized,

	#[display("validation error @ `{field}`")]
	Validation { field: String },

	#[display("invalid parameter @ `{field}`: {value}")]
	InvalidParam { field: String, value: String },

	#[display("invalid value(s) @ parameter `{field}`: {values}")]
	InvalidValues { field: String, values: String },

	#[display("exceeded limit @ parameter `{field}`: {limit}")]
	ExceededLimit { field: String, limit: usize },

	#[display("missing input params")]
	MissingInputParams,

	#[display("could not connect to `{name}`")]
	InvalidService { name: String },

	#[display("duplicate found @ `{field}`: {value}")]
	Duplicate { field: String, value: String },

	#[display("duplicates found @ `{field}`: {values}")]
	Duplicates { field: String, values: String },

	#[display("bad request: {reason}")]
	BadRequest { reason: String },

	#[display("too early: {reason}")]
	TooEarly { reason: String },

	#[display("not found")]
	NotFound,

	#[display("rekt")]
	Internal { error: Report },
}

impl IntoResponse for ServerError {
	fn into_response(self) -> Response {
		let http_code = match self {
			ServerError::NotFound => StatusCode::NOT_FOUND,
			ServerError::Unauthorized => StatusCode::UNAUTHORIZED,
			ServerError::TooEarly { .. } => StatusCode::from_u16(425).unwrap(),
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
