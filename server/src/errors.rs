use axum::{
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use derive_more::{Display, Error};
use eyre::{ErrReport, Report};
use sea_orm::DbErr;
use serde_json::json;
use std::borrow::Cow;

#[derive(Debug, Display, Error)]
pub enum ServerError<'a> {
	#[display("unauthorized")]
	Unauthorized,

	#[display("invalid parameter for `{field}`: {value}")]
	InvalidParam { field: Cow<'a, str>, value: Cow<'a, str> },

	#[display("invalid value(s) for `{field}`: {values}")]
	InvalidValues { field: Cow<'a, str>, values: Cow<'a, str> },

	#[display("missing input params")]
	MissingInputParams,

	#[display("could not connect to `{name}`")]
	InvalidService { name: Cow<'a, str> },

	#[display("duplicate found at `{field}`: {value}")]
	Duplicate { field: Cow<'a, str>, value: Cow<'a, str> },

	#[display("duplicates found at `{field}`: {values}")]
	Duplicates { field: Cow<'a, str>, values: Cow<'a, str> },

	#[display("bad request: {reason}")]
	BadRequest { reason: Cow<'a, str> },

	#[display("too early: {reason}")]
	TooEarly { reason: Cow<'a, str> },

	#[display("not found")]
	NotFound,

	#[display("rekt")]
	Internal { error: Report },
}

impl IntoResponse for ServerError<'static> {
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

impl From<DbErr> for ServerError<'static> {
	fn from(e: DbErr) -> ServerError<'static> {
		ServerError::Internal { error: Report::new(e) }
	}
}

impl From<ErrReport> for ServerError<'static> {
	fn from(e: ErrReport) -> ServerError<'static> {
		ServerError::Internal { error: e }
	}
}

impl From<serde_json::Error> for ServerError<'static> {
	fn from(e: serde_json::Error) -> ServerError<'static> {
		ServerError::Internal { error: Report::new(e) }
	}
}
