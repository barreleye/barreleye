use eyre::Result;
use tracing_subscriber::{fmt, prelude::*};

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

	tracing_subscriber::registry().with(fmt_layer).init();

	Ok(())
}
