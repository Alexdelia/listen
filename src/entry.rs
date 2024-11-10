use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum Source {
	Yt(String),
	Sc(String, String),
}

pub type Q = u8;

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
	pub s: Source,
	pub q: Q,
}
