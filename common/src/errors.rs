use derive_more::{Display, Error};

#[derive(Debug, Clone, Display, Error)]
pub enum AppError<'a> {
	#[display(fmt = "Failed to install signal handler")]
	SignalHandler,

	#[display(fmt = "Invalid config @ `{config}`: {error}")]
	Config { config: &'a str, error: &'a str },

	#[display(fmt = "Could not start server @ `{url}`: {error}")]
	ServerStartup { url: String, error: String },

	#[display(fmt = "Could not connect to storage @ `{url}`")]
	StorageConnection { url: String },

	#[display(fmt = "Could not connect to database @ `{url}`")]
	DatabaseConnection { url: String },

	#[display(fmt = "Could not connect to warehouse @ `{url}`")]
	WarehouseConnection { url: String },

	#[display(fmt = "Could not complete network setup:\n{error}")]
	Network { error: String },

	#[display(fmt = "Could not check sanctions list:\n{error}")]
	SanctionsList { error: String },

	#[display(fmt = "Indexing failed: {error}")]
	Indexing { error: String },

	#[display(fmt = "Unexpected error: {error}")]
	Unexpected { error: String },
}
