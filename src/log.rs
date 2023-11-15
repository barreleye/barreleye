use eyre::Result;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn setup() -> Result<()> {
	color_eyre::install()?;

	let filter = EnvFilter::new("none")
		.add_directive("barreleye_indexer=trace".parse()?)
		.add_directive("barreleye_server=trace".parse()?)
		.add_directive("barreleye=trace".parse()?)
		.add_directive("tower_http::trace=debug".parse()?);

	tracing_subscriber::registry().with(fmt::layer().compact()).with(filter).init();

	Ok(())
}
