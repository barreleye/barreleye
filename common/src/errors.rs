use derive_more::{Display, Error};
use std::borrow::Cow;

#[derive(Debug, Clone, Display, Error)]
pub enum AppError<'a> {
	#[display("Failed to install signal handler")]
	SignalHandler,

	#[display("Invalid config @ `{config}`: {error}")]
	Config { config: Cow<'a, str>, error: Cow<'a, str> },

	#[display("Could not start server @ `{url}`: {error}")]
	ServerStartup { url: Cow<'a, str>, error: Cow<'a, str> },

	#[display("Could not connect to {service} @ `{url}`")]
	Connection { service: Cow<'a, str>, url: Cow<'a, str> },

	#[display("Could not complete network setup:\n{error}")]
	Network { error: Cow<'a, str> },

	#[display("Indexing failed: {error}")]
	Indexing { error: Cow<'a, str> },

	#[display("Unexpected error: {error}")]
	Unexpected { error: Cow<'a, str> },
}
