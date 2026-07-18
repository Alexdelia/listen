use std::{fs, path::PathBuf};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};
use serde::Deserialize;

use crate::{cache, meta_brainz, music_brainz};

use super::super::agent;

use super::Client;

const ENDPOINT: &str = "https://musicbrainz.org/oauth2/token";
pub(super) const REDIRECT_URI: &str = "urn:ietf:wg:oauth:2.0:oob";

const FILE: &str = "musicbrainz-refresh-token";

pub(super) struct Token {
	pub access: String,
	pub refresh: Option<String>,
}

pub(super) fn exchange(client: &Client, code: &str) -> hmerr::Result<Token> {
	request(&[
		("grant_type", "authorization_code"),
		("code", code),
		("client_id", &client.id),
		("client_secret", &client.secret),
		("redirect_uri", REDIRECT_URI),
	])
}

pub(super) fn refresh(client: &Client, refresh_token: &str) -> hmerr::Result<Token> {
	request(&[
		("grant_type", "refresh_token"),
		("refresh_token", refresh_token),
		("client_id", &client.id),
		("client_secret", &client.secret),
	])
}

fn request(form: &[(&str, &str)]) -> hmerr::Result<Token> {
	meta_brainz::block_ready();

	let mut response = agent::build()
		.post(ENDPOINT)
		.header("user-agent", music_brainz::USER_AGENT)
		.send_form(form.iter().copied())
		.map_err(|e| ge!(format!("{R}failed to reach musicbrainz oauth{D}\n{e}")))?;

	let status = response.status();
	let body = response.body_mut().read_to_string().unwrap_or_default();

	if !status.is_success() {
		return Err(ge!(format!(
			"{R}musicbrainz oauth refused the request{D} ({B}{status}{D})\n{body}"
		))
		.into());
	}

	let payload = serde_json::from_str::<Payload>(&body).map_err(|e| {
		ge!(format!(
			"{R}failed to parse musicbrainz oauth response{D}\n{e}"
		))
	})?;

	Ok(Token {
		access: payload.access_token,
		refresh: payload.refresh_token,
	})
}

#[derive(Deserialize)]
struct Payload {
	access_token: String,
	refresh_token: Option<String>,
}

pub(super) fn stored() -> hmerr::Result<Option<String>> {
	let path = path()?;

	if !path.exists() {
		return Ok(None);
	}

	let content = fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	let token = content.trim();

	Ok((!token.is_empty()).then(|| token.to_string()))
}

pub(super) fn store(refresh_token: &str) -> hmerr::Result<()> {
	let path = path()?;
	cache::prepare(&path)?;

	fs::write(&path, refresh_token).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn path() -> hmerr::Result<PathBuf> {
	Ok(cache::root()?.join(FILE))
}
