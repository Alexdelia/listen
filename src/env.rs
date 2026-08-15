use std::sync::OnceLock;

use ansi::abbrev::{B, D, G, M, R, Y};
use hmerr::{ge, ioe};

const DOTENV_FILE: &str = ".env";

const FALSE: [&str; 4] = ["0", "false", "no", "off"];

static DOTENV: OnceLock<()> = OnceLock::new();

#[derive(Clone, Copy)]
pub enum Var {
	SoundcloudClientId,
	MusicBrainzClientId,
	MusicBrainzClientSecret,
	Keep,
}

impl Var {
	pub fn key(self) -> &'static str {
		match self {
			Self::SoundcloudClientId => "SOUNDCLOUD_CLIENT_ID",
			Self::MusicBrainzClientId => "MUSICBRAINZ_CLIENT_ID",
			Self::MusicBrainzClientSecret => "MUSICBRAINZ_CLIENT_SECRET",
			Self::Keep => "DECLARATIVE_LISTEN_KEEP",
		}
	}
}

pub fn load() -> hmerr::Result<()> {
	let Err(e) = dotenvy::dotenv() else {
		return Ok(());
	};

	match e {
		dotenvy::Error::Io(e) => Err(ioe!(
			".env",
			e,
			h:format!("please {B}{G}copy {M}.env.example{D} to {B}{Y}{DOTENV_FILE}{D} and {B}{G}fill in the values{D}")
		))?,
		_ => Err(e.into()),
	}
}

fn read_dotenv() {
	DOTENV.get_or_init(|| {
		let _ = dotenvy::dotenv();
	});
}

pub fn get_opt(key: Var) -> Option<String> {
	read_dotenv();

	std::env::var(key.key()).ok().filter(|v| !v.is_empty())
}

pub fn get_bool(key: Var) -> bool {
	get_opt(key).is_some_and(|value| !FALSE.contains(&value.trim().to_lowercase().as_str()))
}

pub fn get(key: Var) -> hmerr::Result<String> {
	read_dotenv();

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
