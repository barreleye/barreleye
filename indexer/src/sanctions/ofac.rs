use eyre::Result;
use tracing::debug;

use crate::Indexer;

impl Indexer {
	pub async fn check_ofac(&self) -> Result<()> {
		debug!("Syncing with OFAC…");
		Ok(())
	}
}
