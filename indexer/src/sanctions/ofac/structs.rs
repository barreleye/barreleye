use hard_xml::XmlRead;
use std::borrow::Cow;

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "sdnList")]
pub struct SdnList<'a> {
	#[xml(child = "sdnEntry")]
	sdn_entries: Vec<SdnEntry<'a>>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "sdnEntry")]
pub struct SdnEntry<'a> {
	#[xml(child = "uid")]
	uid: Uid<'a>,
	#[xml(child = "sdnType")]
	sdn_type: SdnType<'a>,
	#[xml(child = "firstName")]
	first_name: Option<FirstName<'a>>,
	#[xml(child = "lastName")]
	last_name: Option<LastName<'a>>,
	#[xml(child = "idList")]
	id_list: Option<IdList<'a>>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "uid")]
pub struct Uid<'a> {
	#[xml(text)]
	text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "sdnType")]
pub struct SdnType<'a> {
	#[xml(text)]
	text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "firstName")]
pub struct FirstName<'a> {
	#[xml(text)]
	text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "lastName")]
pub struct LastName<'a> {
	#[xml(text)]
	text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "idList")]
pub struct IdList<'a> {
	#[xml(child = "id")]
	ids: Vec<IdListId<'a>>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "id")]
pub struct IdListId<'a> {
	#[xml(child = "uid")]
	uid: IdListItemUid<'a>,
	#[xml(child = "idType")]
	id_type: IdListItemType<'a>,
	#[xml(child = "idNumber")]
	id_number: Option<IdListItemNumber<'a>>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "uid")]
pub struct IdListItemUid<'a> {
	#[xml(text)]
	text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "idType")]
pub struct IdListItemType<'a> {
	#[xml(text)]
	text: Cow<'a, str>,
}

#[derive(XmlRead, PartialEq, Debug)]
#[xml(tag = "idNumber")]
pub struct IdListItemNumber<'a> {
	#[xml(text)]
	text: Cow<'a, str>,
}
