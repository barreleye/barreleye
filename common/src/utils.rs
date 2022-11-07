use chrono::{offset::Utc, Duration, NaiveDateTime};
use nanoid::nanoid;
use uuid::Uuid;

use crate::IdPrefix;

pub fn new_unique_id(prefix: IdPrefix) -> String {
	unique_id(
		prefix,
		&nanoid!(
			8,
			&[
				'2', '3', '4', '5', '6', '7', '8', '9', 'a', 'c', 'd', 'e',
				'g', 'h', 'j', 'k', 'm', 'n', 'q', 'r', 's', 't', 'v', 'w',
				'x', 'z',
			]
		),
	)
}

pub fn unique_id(prefix: IdPrefix, id: &str) -> String {
	format!("{prefix}_{id}")
}

pub fn new_uuid() -> uuid::Uuid {
	Uuid::new_v4()
}

pub fn now() -> NaiveDateTime {
	Utc::now().naive_utc()
}

pub fn ago_in_seconds(secs: u64) -> NaiveDateTime {
	now() - Duration::seconds(secs as i64)
}
