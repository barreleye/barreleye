use derive_more::{Display, Error};
use std::borrow::Cow;

#[derive(Debug, Clone, Display, Error)]
pub enum AppError<'a> {
	#[display("Failed to install signal handler")]
	SignalHandler,

	#[display("Configuration for `{config}`: {error}")]
	Config { config: Cow<'a, str>, error: Cow<'a, str> },

	#[display("Server startup: {error}")]
	ServerStartup { error: Cow<'a, str> },

	#[display("Could not connect to {service} at \"{url}\"")]
	Connection { service: Cow<'a, str>, url: Cow<'a, str> },

	#[display("Could not complete network setup:\n{error}")]
	Network { error: Cow<'a, str> },

	#[display("Indexing failed: {error}")]
	Indexing { error: Cow<'a, str> },

	#[display("Unexpected error: {error}")]
	Unexpected { error: Cow<'a, str> },
}
