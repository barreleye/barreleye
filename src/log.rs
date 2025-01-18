use eyre::Result;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn setup() -> Result<()> {
	color_eyre::install()?;

	let fmt_layer = fmt::layer()
		.with_target(false)
		.with_level(true)
		.with_line_number(false)
		.with_file(false)
		.with_thread_ids(false)
		.compact()
		.with_writer(std::io::stdout);

	let filter_layer = EnvFilter::try_from_default_env()
		.or_else(|_| EnvFilter::try_new("info"))?
		.add_directive("sea_orm=off".parse()?);

	tracing_subscriber::registry().with(filter_layer).with(fmt_layer).init();

	Ok(())
}
