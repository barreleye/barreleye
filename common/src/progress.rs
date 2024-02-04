use console::style;
use tracing::warn;

use crate::{
	Warnings, EMOJI_MIGRATIONS, EMOJI_NETWORKS, EMOJI_READY, EMOJI_SETUP,
};

pub enum ReadyType {
	All(String),
	Server(String),
	Indexer,
}

pub enum Step {
	Setup,
	Migrations,
	Networks,
	Ready(ReadyType, Warnings),
}

#[derive(Clone)]
pub struct Progress {
	with_indexer: bool,
}

impl Progress {
	pub fn new(with_indexer: bool) -> Self {
		Self { with_indexer }
	}

	pub fn show(&self, step: Step) {
		let total_steps = if self.with_indexer { 4 } else { 3 };

		let out = |step, emoji, text| {
			println!(
				"{} {}{}",
				style(format!("[{step}/{total_steps}]")).bold().dim(),
				emoji,
				text
			)
		};

		match step {
			Step::Setup => out(1, EMOJI_SETUP, "Initializing…"),
			Step::Migrations => out(2, EMOJI_MIGRATIONS, "Running migrations…"),
			Step::Networks => out(3, EMOJI_NETWORKS, "Connecting to networks…"),
			Step::Ready(ready_type, warnings) => {
				out(total_steps, EMOJI_READY, "Starting up…");
				fn show_status(status: &str) {
					println!(
						"          {} {}",
						style("↳").bold().dim(),
						style(status).bold().dim(),
					);
				}
				match ready_type {
					ReadyType::All(addr) => {
						show_status("Indexer enabled");
						show_status(&format!("Listening on {addr}…\n"));
					}
					ReadyType::Server(addr) => {
						show_status("Indexer disabled");
						show_status(&format!("Listening on {addr}…\n"));
					}
					ReadyType::Indexer => {
						show_status("Indexer enabled");
						show_status("Server disabled\n");
					}
				};

				for warning in warnings.into_iter() {
					warn!(warning);
				}
			}
		}
	}
}
