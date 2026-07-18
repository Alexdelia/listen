pub mod parse;

use serde::{Deserialize, Deserializer, Serialize, de::Error};

pub type Source = uuid::Uuid;
pub type Q = u8;

pub const Q_MAX: Q = 4;

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
	pub s: Source,
	#[serde(deserialize_with = "bounded_q")]
	pub q: Q,
	pub playlist: Vec<String>,
}

fn bounded_q<'de, D>(deserializer: D) -> Result<Q, D::Error>
where
	D: Deserializer<'de>,
{
	let q = Q::deserialize(deserializer)?;

	if q > Q_MAX {
		return Err(D::Error::custom(format!("q{q} out of range 0..={Q_MAX}")));
	}

	Ok(q)
}
