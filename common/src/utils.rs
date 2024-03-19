use chrono::{offset::Utc, Duration, NaiveDateTime};
use directories::ProjectDirs;
use governor::Quota;
use nanoid::nanoid;
use sha2::{Digest, Sha256};
use std::{num::NonZeroU32, path::PathBuf, sync::Arc};
use url::Url;
use uuid::Uuid;

use crate::{GovernorRateLimiter, IdPrefix, RateLimiter};

pub fn sha256(input: &str) -> Vec<u8> {
	let mut hasher = Sha256::new();
	hasher.update(input.as_bytes());
	hasher.finalize().to_vec()
}

pub fn project_dir(folder: Option<&str>) -> PathBuf {
	// @TODO will panic on systems with no home directory
	ProjectDirs::from("com", "barreleye", "barreleye")
		.map(|d| {
			let mut ret = PathBuf::from(d.data_dir());
			if let Some(folder) = folder {
				ret = ret.join(folder);
			}

			ret
		})
		.unwrap()
}

pub fn get_rate_limiter(rps: u32) -> Option<Arc<RateLimiter>> {
	NonZeroU32::new(rps).map(|non_zero_rps| {
		Arc::new(GovernorRateLimiter::direct(Quota::per_second(non_zero_rps)))
	})
}

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
	now() - Duration::try_seconds(secs as i64).unwrap()
}

pub fn with_masked_auth(url: &str) -> String {
	match Url::parse(url) {
		Ok(mut parsed_url) => {
			if parsed_url.password().is_some() {
				parsed_url.set_password(Some("***")).ok();
			}

			parsed_url.to_string()
		}
		_ => url.to_string(),
	}
}

pub fn without_pathname(url: &str) -> (String, String) {
	match Url::parse(url) {
		Ok(mut parsed_url) => {
			let path = parsed_url.path().trim_matches('/').to_string();
			parsed_url.set_path("");

			(parsed_url.to_string(), path)
		}
		_ => (url.to_string(), "".to_string()),
	}
}

pub fn get_db_path(url: &str) -> String {
	if let Ok(parsed_url) = Url::parse(url) {
		if let Some(host) = parsed_url.host() {
			return host.to_string();
		} else {
			return parsed_url.path().trim_end_matches('/').to_string();
		}
	}

	"".to_string()
}

pub fn has_pathname(url: &str) -> bool {
	if let Ok(parsed_url) = Url::parse(url) {
		!parsed_url.path().to_string().is_empty()
	} else {
		false
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	#[test]
	fn test_with_masked_auth() {
		let data = HashMap::from([
			("", ""),
			("http://test.com/", "http://test.com/"),
			("http://username@test.com/", "http://username@test.com/"),
			(
				"http://username:password@test.com/",
				"http://username:***@test.com/",
			),
		]);

		for (from, to) in data.into_iter() {
			assert_eq!(with_masked_auth(&from), to)
		}
	}

	#[test]
	fn test_without_pathname() {
		let data = HashMap::from([
			("", ("", "")),
			("http://test.com/pathname", ("http://test.com/", "pathname")),
			(
				"http://username@test.com/pathname",
				("http://username@test.com/", "pathname"),
			),
			(
				"http://username:password@test.com/pathname",
				("http://username:password@test.com/", "pathname"),
			),
		]);

		for (from, (to, pathname)) in data.into_iter() {
			assert_eq!(
				without_pathname(&from),
				(to.to_string(), pathname.to_string())
			)
		}
	}

	#[test]
	fn test_get_db_path() {
		let data = HashMap::from([
			("", ""),
			("protocol://test", "test"),
			("protocol:///test", "/test"),
			("protocol:///test?params", "/test"),
		]);

		for (from, path) in data.into_iter() {
			assert_eq!(get_db_path(&from), path.to_string())
		}
	}
}
