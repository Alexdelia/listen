use console::style;

use crate::env::{self, Var};

pub(crate) struct Client {
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
			"{}{}{}",
			style("invalid ").yellow(),
			style(&key.key()).bold().yellow(),
			style(", nothing is submitted to musicbrainz").yellow()
		);
		return None;
	}

	Some(value)
}
