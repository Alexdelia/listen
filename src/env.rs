use ansi::abbrev::{B, D, G, M, R, Y};
use hmerr::{ge, ioe};

const DOTENV_FILE: &str = ".env";

pub enum Var {
	SoundcloudClientId,
}

impl Var {
	pub fn key(&self) -> &'static str {
		match self {
			Self::SoundcloudClientId => "SOUNDCLOUD_CLIENT_ID",
		}
	}
}

pub fn load() -> hmerr::Result<()> {
	let Err(e) = dotenv::dotenv() else {
		return Ok(());
	};

	match e {
		dotenv::Error::Io(e) => Err(ioe!(
			".env",
			e,
			h:format!("please {B}{G}copy {M}.env.example{D} to {B}{Y}{DOTENV_FILE}{D} and {B}{G}fill in the values{D}")
		))?,
		_ => return Err(e.into()),
	}
}

pub fn get(key: Var) -> hmerr::Result<String> {
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
