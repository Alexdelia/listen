use ansi::abbrev::{B, D, Y};

use crate::env::{self, Var};

pub(super) struct Client {
	pub id: String,
	pub secret: String,
}

impl Client {
	pub(super) fn from_env() -> Option<Self> {
		Some(Self {
			id: credential(Var::MusicBrainzClientId)?,
			secret: credential(Var::MusicBrainzClientSecret)?,
		})
	}
}

fn credential(key: Var) -> Option<String> {
	let value = env::get_opt(key)?;

	if value.contains(char::is_whitespace) {
		eprintln!(
			"{Y}invalid {B}{key}{D}{Y}, rating sync skipped{D}",
			key = key.key()
		);
		return None;
	}

	Some(value)
}
