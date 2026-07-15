pub mod parse;

use serde::{Deserialize, Serialize};

pub type Source = String;
pub type Q = u8;

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
	pub s: Source,
	pub q: Q,
	pub playlist: Vec<String>,
}
