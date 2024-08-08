use derive_more::Display;
use eyre::{ErrReport, Result};
use std::str::FromStr;
use url::Url;

#[derive(Display, Debug, Clone, Default, PartialEq, Eq)]
pub enum Service {
	#[default]
	#[display("Unknown")]
	Unknown,
	#[display("S3")]
	S3,
	#[display("S3-compatible")]
	S3Compatible,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct S3 {
	pub service: Service,
	pub url: String,
	pub region: Option<String>,
	pub domain: Option<String>,
	pub bucket: Option<String>,
}

impl FromStr for S3 {
	type Err = ErrReport;

	fn from_str(s: &str) -> Result<Self> {
		let mut ret = S3 { ..Default::default() };

		let parsed_url = Url::parse(s)?;
		ret.url = parsed_url.to_string();
		if let Some(domain) = parsed_url.domain() {
			let parts: Vec<String> =
				domain.split('.').map(|v| v.to_string()).collect();
			if parts.len() >= 3 && parts[parts.len() - 2] == "amazonaws" {
				ret.service = Service::S3;
				ret.region = Some(parts[parts.len() - 3].clone());
			} else {
				ret.service = Service::S3Compatible;
				ret.domain = Some(domain.to_string());
			}

			if let Some(mut segments) = parsed_url.path_segments() {
				if let Some(bucket) = segments.next() {
					if !bucket.is_empty() {
						ret.bucket = Some(bucket.to_string());
					}
				}
			}
		}

		Ok(ret)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	#[test]
	fn test_s3() {
		let data = HashMap::from([
			(
				"http://s3.us-east-1.amazonaws.com/bucket_name/",
				S3 {
					service: Service::S3,
					url: "http://s3.us-east-1.amazonaws.com/bucket_name/"
						.to_string(),
					region: Some("us-east-1".to_string()),
					domain: None,
					bucket: Some("bucket_name".to_string()),
				},
			),
			(
				"http://storage.googleapis.com/bucket_name/",
				S3 {
					service: Service::S3Compatible,
					url: "http://storage.googleapis.com/bucket_name/"
						.to_string(),
					region: None,
					domain: Some("storage.googleapis.com".to_string()),
					bucket: Some("bucket_name".to_string()),
				},
			),
			(
				"http://example.com/bucket_name/",
				S3 {
					service: Service::S3Compatible,
					url: "http://example.com/bucket_name/".to_string(),
					region: None,
					domain: Some("example.com".to_string()),
					bucket: Some("bucket_name".to_string()),
				},
			),
			(
				"http://example.com/one/two/three",
				S3 {
					service: Service::S3Compatible,
					url: "http://example.com/one/two/three".to_string(),
					region: None,
					domain: Some("example.com".to_string()),
					bucket: Some("one".to_string()),
				},
			),
			(
				"http://s3.us-east-1.amazonaws.com/",
				S3 {
					service: Service::S3,
					url: "http://s3.us-east-1.amazonaws.com/".to_string(),
					region: Some("us-east-1".to_string()),
					domain: None,
					bucket: None,
				},
			),
		]);

		for (url, s3) in data.into_iter() {
			assert_eq!(S3::from_str(&url).unwrap(), s3);
		}
	}
}
