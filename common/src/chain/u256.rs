pub use ethers::types::U256;
use serde::{
	de::{Deserialize, Deserializer},
	ser::{Serialize, Serializer},
};

pub fn serialize<S: Serializer>(u: &U256, serializer: S) -> Result<S::Ok, S::Error> {
	let mut buf: [u8; 32] = [0; 32];
	u.to_little_endian(&mut buf);
	buf.serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
	D: Deserializer<'de>,
{
	let u: [u8; 32] = Deserialize::deserialize(deserializer)?;
	Ok(U256::from_little_endian(&u))
}
