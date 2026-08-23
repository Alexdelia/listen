use std::sync::OnceLock;

use ansi::abbrev::{B, D, G, M, R, Y};
use hmerr::ge;

const DOTENV_FILE: &str = ".env";

static DOTENV: OnceLock<Option<String>> = OnceLock::new();

#[derive(Clone, Copy)]
pub(crate) enum Var {
	SoundcloudClientId,
	MusicBrainzClientId,
	MusicBrainzClientSecret,
}

impl Var {
	pub(crate) const fn key(self) -> &'static str {
		match self {
			Self::SoundcloudClientId => "SOUNDCLOUD_CLIENT_ID",
			Self::MusicBrainzClientId => "MUSICBRAINZ_CLIENT_ID",
			Self::MusicBrainzClientSecret => "MUSICBRAINZ_CLIENT_SECRET",
		}
	}
}

pub(crate) fn read() {
	let _ = complaint();
}

pub(crate) fn load() -> hmerr::Result<()> {
	let Some(complaint) = complaint() else {
		return Ok(());
	};

	Err(ge!(
		format!("{B}{R}{DOTENV_FILE}{D}: {complaint}"),
		h: format!("please {B}{G}copy {M}.env.example{D} to {B}{Y}{DOTENV_FILE}{D} and {B}{G}fill in the values{D}")
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

pub(crate) fn get(key: Var) -> hmerr::Result<String> {
	read();

	let key = key.key();

	match std::env::var(key) {
		Ok(val) => Ok(val),
		Err(e) => Err(ge!(
			format!("{B}{R}{key}{D} does not exist in {B}{M}{DOTENV_FILE}{D}"),
			h: format!("add {B}{G}{key}=\"your value\"{D} to {B}{M}{DOTENV_FILE}{D}"),
			s: e,
		))?,
	}
}
