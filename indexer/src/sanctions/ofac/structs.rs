use hard_xml::XmlRead;
use serde::Deserialize;
use serde_json::value::Value as Json;
use std::{borrow::Cow, collections::HashMap};

#[derive(Deserialize)]
pub struct EntityJsonData {
	pub uid: String,
}

#[derive(Deserialize)]
pub struct AddressJsonData {
	pub uid: String,
}

#[derive(Debug)]
pub struct ListEntity {
	pub uid: String,
	pub name: String,
	pub description: String,
	pub data: Json,
	pub addresses: HashMap<String, ListEntityAddress>,
}

#[derive(Debug)]
pub struct ListEntityAddress {
	pub uid: String,
	pub symbol: String,
	pub address: String,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "sdnList")]
pub struct SdnList<'a> {
	#[xml(child = "sdnEntry")]
	pub sdn_entries: Vec<SdnEntry<'a>>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "sdnEntry")]
pub struct SdnEntry<'a> {
	#[xml(child = "uid")]
	pub uid: Uid<'a>,
	#[xml(child = "sdnType")]
	pub sdn_type: SdnType<'a>,
	#[xml(child = "firstName")]
	pub first_name: Option<FirstName<'a>>,
	#[xml(child = "lastName")]
	pub last_name: Option<LastName<'a>>,
	#[xml(child = "idList")]
	pub id_list: Option<IdList<'a>>,
}

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "uid")]
pub struct Uid<'a> {
	#[xml(text)]
	pub text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "sdnType")]
pub struct SdnType<'a> {
	#[xml(text)]
	pub text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "firstName")]
pub struct FirstName<'a> {
	#[xml(text)]
	pub text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "lastName")]
pub struct LastName<'a> {
	#[xml(text)]
	pub text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "idList")]
pub struct IdList<'a> {
	#[xml(child = "id")]
	pub ids: Vec<IdListId<'a>>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "id")]
pub struct IdListId<'a> {
	#[xml(child = "uid")]
	pub uid: IdListItemUid<'a>,
	#[xml(child = "idType")]
	pub id_type: IdListItemType<'a>,
	#[xml(child = "idNumber")]
	pub id_number: Option<IdListItemNumber<'a>>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "uid")]
pub struct IdListItemUid<'a> {
	#[xml(text)]
	pub text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "idType")]
pub struct IdListItemType<'a> {
	#[xml(text)]
	pub text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "idNumber")]
pub struct IdListItemNumber<'a> {
	#[xml(text)]
	pub text: Cow<'a, str>,
}
