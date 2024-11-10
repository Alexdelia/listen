use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
enum Source {
	Yt(String),
	Sc(String, String),
}

pub type Q = u8;

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
	s: Source,
	q: Q,
}
