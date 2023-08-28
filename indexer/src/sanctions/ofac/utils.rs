use eyre::Result;
use serde_json::json;
use std::collections::HashMap;

use super::structs::{ListEntity, ListEntityAddress, SdnList};
use crate::Indexer;

impl Indexer {
	pub fn parse_xml_data(&self, xml_data: SdnList) -> Result<HashMap<String, ListEntity>> {
		let mut ret = HashMap::new();

		for sdn_entry in xml_data.sdn_entries {
			let uid = sdn_entry.uid.clone().text.to_string();

			let name = match (sdn_entry.first_name.clone(), sdn_entry.last_name.clone()) {
				(Some(first_name), Some(last_name)) => {
					format!("{} {}", first_name.text, last_name.text)
				}
				(None, Some(last_name)) => format!("{}", last_name.text),
				_ => "".to_string(),
			};

			let addresses = {
				let mut ret = HashMap::new();
				let id_type_prefix = "Digital Currency Address - ";

				if let Some(id_list) = sdn_entry.id_list {
					for id in id_list.ids {
						if id.id_type.text.starts_with(id_type_prefix) {
							let symbol = id.id_type.text[id_type_prefix.len()..].trim().to_string();
							if symbol.is_empty() {
								continue;
							}

							if let Some(id_number) = id.id_number {
								let uid = id.uid.text.to_string();
								let address = id_number.text.to_string();

								ret.insert(uid.clone(), ListEntityAddress { uid, symbol, address });
							}
						}
					}
				}

				ret
			};

			if !addresses.is_empty() {
				let t = sdn_entry.sdn_type.text.to_string();

				let first_name = match sdn_entry.first_name {
					Some(v) => v.text.to_string(),
					_ => "".to_string(),
				};

				let last_name = match sdn_entry.last_name {
					Some(v) => v.text.to_string(),
					_ => "".to_string(),
				};

				ret.insert(
					uid.clone(),
					ListEntity {
						uid: uid.clone(),
						name,
						description: "".to_string(),
						data: json!({
							"uid": uid,
							"type": t,
							"firstName": first_name,
							"lastName": last_name,
						}),
						addresses,
					},
				);
			}
		}

		Ok(ret)
	}
}
