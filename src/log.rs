use eyre::Result;
use std::env;
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

	let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

	let filter_layer = EnvFilter::new("off")
		.add_directive(format!("barreleye={rust_log}").parse()?)
		.add_directive(format!("barreleye_common={rust_log}").parse()?)
		.add_directive(format!("barreleye_indexer={rust_log}").parse()?)
		.add_directive(format!("barreleye_server={rust_log}").parse()?);

	tracing_subscriber::registry().with(filter_layer).with(fmt_layer).init();

	Ok(())
}
