use std::path::PathBuf;

use listen_cache::json;
use miette::miette;
use serde::{Deserialize, Serialize};

use crate::{cache, meta_brainz::Sent, music_brainz};

use super::{Client, login::SCOPE};

const ENDPOINT: &str = "https://musicbrainz.org/oauth2/token";
pub(super) const REDIRECT_URI: &str = "urn:ietf:wg:oauth:2.0:oob";

const FILE: &str = "musicbrainz-refresh-token";

const INVALID_GRANT: &str = "invalid_grant";

const FAILURE: &str = "failed to reach musicbrainz oauth";

pub(super) struct Token {
	pub access: String,
	pub refresh: Option<String>,
}

pub(super) enum Refresh {
	Token(Token),
	Rejected,
}

enum Reply {
	Token(Token),
	Refused {
		invalid_grant: bool,
		error: miette::Report,
	},
}

pub(super) fn exchange(client: &Client, code: &str) -> miette::Result<Token> {
	match request(&[
		("grant_type", "authorization_code"),
		("code", code),
		("client_id", &client.id),
		("client_secret", &client.secret),
		("redirect_uri", REDIRECT_URI),
	])? {
		Reply::Token(token) => Ok(token),
		Reply::Refused { error, .. } => Err(error),
	}
}

pub(super) fn refresh(client: &Client, refresh_token: &str) -> miette::Result<Refresh> {
	match request(&[
		("grant_type", "refresh_token"),
		("refresh_token", refresh_token),
		("client_id", &client.id),
		("client_secret", &client.secret),
	])? {
		Reply::Token(token) => Ok(Refresh::Token(token)),
		Reply::Refused {
			invalid_grant: true,
			..
		} => Ok(Refresh::Rejected),
		Reply::Refused { error, .. } => Err(error),
	}
}

fn request(form: &[(&str, &str)]) -> miette::Result<Reply> {
	let Sent { status, body } = music_brainz::post_once(
		ENDPOINT,
		|request| request.send_form(form.iter().copied()),
		FAILURE,
	)?;

	if !status.is_success() {
		return Ok(Reply::Refused {
			invalid_grant: is_invalid_grant(&body),
			error: miette!("musicbrainz oauth refused the request ({status})\n{body}"),
		});
	}

	let payload = serde_json::from_str::<Payload>(&body)
		.map_err(|e| miette!("failed to parse musicbrainz oauth response\n{e}"))?;

	Ok(Reply::Token(Token {
		access: payload.access_token,
		refresh: payload.refresh_token,
	}))
}

fn is_invalid_grant(body: &str) -> bool {
	serde_json::from_str::<ErrorPayload>(body).is_ok_and(|e| e.error == INVALID_GRANT)
}

#[derive(Deserialize)]
struct Payload {
	access_token: String,
	refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct ErrorPayload {
	error: String,
}

#[derive(Serialize, Deserialize)]
struct Stored {
	scope: String,
	refresh: String,
}

pub(super) fn stored() -> miette::Result<Option<String>> {
	Ok(json::read::<Stored>(&path()?)?
		.filter(|stored| stored.scope == SCOPE)
		.map(|stored| stored.refresh))
}

pub(super) fn store(refresh_token: &str) -> miette::Result<()> {
	json::write(
		&path()?,
		&Stored {
			scope: SCOPE.to_owned(),
			refresh: refresh_token.to_owned(),
		},
	)
}

fn path() -> miette::Result<PathBuf> {
	Ok(cache::root()?.join(FILE).with_extension(json::EXT))
}
