use eyre::Result;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

pub fn setup() -> Result<()> {
	color_eyre::install()?;

	let filter = EnvFilter::new("none")
		.add_directive("barreleye_indexer=debug".parse()?)
		.add_directive("barreleye_server=debug".parse()?)
		.add_directive("barreleye=debug".parse()?)
		.add_directive("tower_http::trace=debug".parse()?);

	tracing_subscriber::registry().with(fmt::layer().compact()).with(filter).init();

	Ok(())
}
