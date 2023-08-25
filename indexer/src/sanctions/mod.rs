use eyre::Result;
use tokio::task::JoinSet;

use crate::Indexer;
use barreleye_common::Sanctions;

mod ofac;
mod ofsi;

impl Indexer {
	pub async fn check_sanction_lists(&self) -> Result<()> {
		if !self.app.settings.sanction_lists.is_empty() {
			let mut set = JoinSet::new();

			if self.app.settings.sanction_lists.contains(&Sanctions::Ofac) {
				set.spawn({
					let s = self.clone();
					async move { s.check_ofac().await }
				});
			}
			if self.app.settings.sanction_lists.contains(&Sanctions::Ofsi) {
				set.spawn({
					let s = self.clone();
					async move { s.check_ofsi().await }
				});
			}

			while let Some(res) = set.join_next().await {
				res??;
			}
		}

		Ok(())
	}
}
