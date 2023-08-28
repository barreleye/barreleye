use bitcoin::hashes::{sha256, Hash};
use eyre::Result;
use hard_xml::XmlRead;
use tracing::debug;

use crate::Indexer;
use barreleye_common::{
	models::{BasicModel, Config, ConfigKey, Tag},
	utils, RiskLevel, Sanctions,
};
use structs::SdnList;

mod structs;

impl Indexer {
	pub async fn check_ofac(&self) -> Result<()> {
		debug!("Syncing with OFAC…");

		// upsert tag
		let tag_id = utils::get_sanctions_tag_id(&Sanctions::Ofac);
		let _tag = match Tag::get_by_id(self.app.db(), &tag_id).await? {
			Some(tag) => tag,
			_ => {
				let tag_id = Tag::create(
					self.app.db(),
					Tag::new_model(Some(tag_id), "OFAC", RiskLevel::Critical, true),
				)
				.await?;
				Tag::get(self.app.db(), tag_id).await?.unwrap()
			}
		};

		// check if new changes have been published
		let url = "https://www.treasury.gov/ofac/downloads/sdn.xml";
		let xml = reqwest::get(url).await?.text().await?;
		let hash = sha256::Hash::hash(xml.as_bytes()).to_string();
		match Config::get::<_, String>(self.app.db(), ConfigKey::SanctionsChecksumOfac).await? {
			Some(data) if data.value == hash => return Ok(()),
			_ => {}
		}

		// parse data
		let data = SdnList::from_str(&xml);
		println!("data {:?}", data);

		// transform

		// save config
		// Config::set::<_, String>(self.app.db(), ConfigKey::SanctionsChecksumOfac, hash).await?;

		Ok(())
	}
}
