use derive_more::{Display, Error};
use std::borrow::Cow;

#[derive(Debug, Clone, Display, Error)]
pub enum AppError<'a> {
	#[display("failed to install signal handler")]
	SignalHandler,

	#[display("configuration for `{config}`: {error}")]
	Config { config: Cow<'a, str>, error: Cow<'a, str> },

	#[display("server startup: {error}")]
	ServerStartup { error: Cow<'a, str> },

	#[display("could not connect to {service} at \"{url}\"")]
	Connection { service: Cow<'a, str>, url: Cow<'a, str> },

	#[display("could not connect to {service} at \"{url}\" (with credentials)")]
	ConnectionWithCredentials { service: Cow<'a, str>, url: Cow<'a, str> },

	#[display("database: {error}")]
	Database { error: Cow<'a, str> },

	#[display("warehouse: {error}")]
	Warehouse { error: Cow<'a, str> },

	#[display("could not complete network setup:\n{error}")]
	Network { error: Cow<'a, str> },

	#[display("indexing failed: {error}")]
	Indexing { error: Cow<'a, str> },

	#[display("unexpected error: {error}")]
	Unexpected { error: Cow<'a, str> },
}
