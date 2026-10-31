use std::sync::OnceLock;

use miette::miette;

const DOTENV_FILE: &str = ".env";

static DOTENV: OnceLock<Option<String>> = OnceLock::new();

#[derive(Clone, Copy)]
pub(crate) enum Var {
	SoundcloudClientId,
	MusicBrainzClientId,
	MusicBrainzClientSecret,
	ComputeTagSuggestion,
}

impl Var {
	pub(crate) const fn key(self) -> &'static str {
		match self {
			Self::SoundcloudClientId => "SOUNDCLOUD_CLIENT_ID",
			Self::MusicBrainzClientId => "MUSICBRAINZ_CLIENT_ID",
			Self::MusicBrainzClientSecret => "MUSICBRAINZ_CLIENT_SECRET",
			Self::ComputeTagSuggestion => "DECLARATIVE_LISTEN_COMPUTE_TAG_SUGGESTION",
		}
	}
}

pub(crate) fn read() {
	let _ = complaint();
}

pub(crate) fn load() -> miette::Result<()> {
	let Some(complaint) = complaint() else {
		return Ok(());
	};

	Err(miette!(
		help = format!("please copy .env.example to {DOTENV_FILE} and fill in the values"),
		"{DOTENV_FILE}: {complaint}"
	))?
}

fn complaint() -> Option<&'static String> {
	DOTENV
		.get_or_init(|| dotenvy::dotenv().err().map(|e| e.to_string()))
		.as_ref()
}

pub(crate) fn get_opt(key: Var) -> Option<String> {
	read();

	std::env::var(key.key()).ok().filter(|v| !v.is_empty())
}

pub(crate) fn enabled(key: Var) -> bool {
	read();

	listen_cache::flag::enabled(key.key())
}

pub(crate) fn get(key: Var) -> miette::Result<String> {
	read();

	let key = key.key();

	match std::env::var(key) {
		Ok(val) => Ok(val),
		Err(e) => Err(miette!(
			help = format!("add {key}=\"your value\" to {DOTENV_FILE}"),
			"{key} does not exist in {DOTENV_FILE}: {e}"
		)),
	}
}
