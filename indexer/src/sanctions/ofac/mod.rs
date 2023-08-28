use bitcoin::hashes::{sha256, Hash};
use eyre::Result;
use hard_xml::XmlRead;
use sea_orm::ColumnTrait;
use std::collections::HashMap;
use tracing::debug;

use crate::Indexer;
use barreleye_common::{
	models::{
		set, Address, AddressActiveModel, AddressColumn, BasicModel, Config, ConfigKey, Entity,
		EntityActiveModel, EntityColumn, EntityTag, PrimaryId, Tag,
	},
	quit, utils as common_utils, AppError, RiskLevel, Sanctions,
};
use structs::{AddressJsonData, EntityJsonData, SdnList};

mod structs;
mod utils;

impl Indexer {
	pub async fn check_ofac(&self) -> Result<()> {
		debug!("Syncing with OFAC…");

		// check if new changes have been published
		let url = "https://www.treasury.gov/ofac/downloads/sdn.xml";
		let xml = reqwest::get(url).await?.text().await?;
		let hash = sha256::Hash::hash(xml.as_bytes()).to_string();
		match Config::get::<_, String>(self.app.db(), ConfigKey::SanctionsChecksumOfac).await? {
			Some(data) if data.value == hash => {
				debug!("OFAC already up-to-date");
				return Ok(());
			}
			_ => {}
		}

		// exit if can't parse data
		let xml_data = match SdnList::from_str(&xml) {
			Ok(data) if !data.sdn_entries.is_empty() => data,
			_ => quit(AppError::SanctionsList {
				error: "Could not retrieve OFAC sanctions list".to_string(),
			}),
		};

		// upsert tag
		let tag_id = common_utils::get_sanctions_tag_id(&Sanctions::Ofac);
		let tag = match Tag::get_by_id(self.app.db(), &tag_id).await? {
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

		// create a map of existing entities (entity_uid => JoinedEntity)
		let mut entity_map = HashMap::new();
		let mut tmp_entity_ids = vec![];
		for joined_entity in
			Entity::get_all_by_tag_ids(self.app.db(), vec![tag.tag_id].into(), Some(false)).await?
		{
			tmp_entity_ids.push(joined_entity.entity_id);

			let json_data: EntityJsonData = serde_json::from_value(joined_entity.data.clone())?;
			entity_map.insert(json_data.uid, joined_entity);
		}

		// create a map of existing addresses (entity_id => HashMap<address_uid, Address>)
		let mut address_map: HashMap<PrimaryId, HashMap<String, Address>> = HashMap::new();
		for address in
			Address::get_all_by_entity_ids(self.app.db(), tmp_entity_ids.into(), Some(false))
				.await?
		{
			let json_data: AddressJsonData = serde_json::from_value(address.data.clone())?;
			if let Some(map) = address_map.get_mut(&address.entity_id) {
				map.insert(json_data.uid, address);
			} else {
				address_map.insert(address.entity_id, HashMap::from([(json_data.uid, address)]));
			}
		}

		// parse data
		let parsed_xml_data = self.parse_xml_data(xml_data)?;

		// loop through existing entities, if not found in latest data -> mark for deletion
		let deleted = {
			let mut entity_ids_to_delete = vec![];
			let mut entity_uids_to_unmap = vec![];

			for (uid, joined_entity) in entity_map.clone() {
				if !parsed_xml_data.contains_key(&uid) {
					entity_ids_to_delete.push(joined_entity.entity_id);
					entity_uids_to_unmap.push(uid);
				}
			}

			if !entity_ids_to_delete.is_empty() {
				// soft-delete all associated addresses
				Address::update_all_where(
					self.app.db(),
					AddressColumn::EntityId.is_in(entity_ids_to_delete.clone()),
					AddressActiveModel { is_deleted: set(true), ..Default::default() },
				)
				.await?;

				// soft-delete all entities
				Entity::update_all_where(
					self.app.db(),
					EntityColumn::EntityId.is_in(entity_ids_to_delete.clone()),
					EntityActiveModel { is_deleted: set(true), ..Default::default() },
				)
				.await?;

				// clean up maps
				for entity_uid_to_unmap in entity_uids_to_unmap {
					entity_map.remove(&entity_uid_to_unmap);
				}
				for entity_id_to_delete in entity_ids_to_delete.clone() {
					address_map.remove(&entity_id_to_delete);
				}
			}

			entity_ids_to_delete.len()
		};

		// loop through latest data, if not found in existing entities -> create data
		let inserted = {
			let mut inserted = 0;

			for (uid, xml_entity) in parsed_xml_data {
				if !entity_map.contains_key(&uid) {
					inserted += 1;

					// create a new entity
					let entity_id = Entity::create(
						self.app.db(),
						Entity::new_model(
							None,
							Some(xml_entity.name),
							&xml_entity.description,
							Some(xml_entity.data),
							true,
						),
					)
					.await?;

					// apply the tag
					EntityTag::create(self.app.db(), EntityTag::new_model(entity_id, tag.tag_id))
						.await?;

					// create new addresses
					let mut addresses = vec![];
					for (_, address) in xml_entity.addresses {
						addresses.push(Address::new_model(
							None,
							entity_id,
							1,       // @TODO id
							"net_?", // @TODO network_id
							&address.address,
							"",
							None,
							true,
						));
					}
				}
			}

			inserted
		};

		// @TODO
		let updated = 0;

		// save config
		// Config::set::<_, String>(self.app.db(), ConfigKey::SanctionsChecksumOfac, hash).await?;

		// report stats
		debug!(inserted = inserted, updated = updated, deleted = deleted);

		Ok(())
	}
}
