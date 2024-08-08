use derive_more::{Display, Error};

#[derive(Debug, Clone, Display, Error)]
pub enum AppError<'a> {
	#[display("Failed to install signal handler")]
	SignalHandler,

	#[display("Invalid config @ `{config}`: {error}")]
	Config { config: &'a str, error: &'a str },

	#[display("Could not start server @ `{url}`: {error}")]
	ServerStartup { url: String, error: String },

	#[display("Could not connect to storage @ `{url}`")]
	StorageConnection { url: String },

	#[display("Could not connect to database @ `{url}`")]
	DatabaseConnection { url: String },

	#[display("Could not connect to warehouse @ `{url}`")]
	WarehouseConnection { url: String },

	#[display("Could not complete network setup:\n{error}")]
	Network { error: String },

	#[display("Indexing failed: {error}")]
	Indexing { error: String },

	#[display("Unexpected error: {error}")]
	Unexpected { error: String },
}
