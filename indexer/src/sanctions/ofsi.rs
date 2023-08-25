use eyre::Result;
use tracing::debug;

use crate::Indexer;

impl Indexer {
	pub async fn check_ofsi(&self) -> Result<()> {
		debug!("Syncing with OFSI…");
		Ok(())
	}
}
