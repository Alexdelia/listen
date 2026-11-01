use std::{
	fs, io,
	path::{Path, PathBuf},
	time::Duration,
};

use listen_cache::json;
use miette::{IntoDiagnostic, WrapErr, miette};
use serde::{Deserialize, Serialize};

use crate::{cache, meta_brainz::Sent, music_brainz};

use super::{Client, SCOPE};

const ENDPOINT: &str = "https://musicbrainz.org/oauth2/token";
pub(super) const REDIRECT_URI: &str = "urn:ietf:wg:oauth:2.0:oob";

const FILE: &str = "musicbrainz-refresh-token";

const INVALID_GRANT: &str = "invalid_grant";

const FAILURE: &str = "failed to reach musicbrainz oauth";

pub(super) struct Token {
	pub access: String,
	pub refresh: Option<String>,
	pub lasts: Option<Duration>,
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
		lasts: payload.expires_in.map(Duration::from_secs),
	}))
}

fn is_invalid_grant(body: &str) -> bool {
	serde_json::from_str::<ErrorPayload>(body).is_ok_and(|e| e.error == INVALID_GRANT)
}

#[derive(Deserialize)]
struct Payload {
	access_token: String,
	refresh_token: Option<String>,
	expires_in: Option<u64>,
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

pub(super) enum Kept {
	Refresh(String),
	Outscoped,
	Nothing,
}

pub(super) fn stored() -> miette::Result<Kept> {
	Ok(kept(
		json::read::<Stored>(&path()?)?,
		superseded_path()?.exists(),
	))
}

fn kept(stored: Option<Stored>, superseded: bool) -> Kept {
	match stored {
		Some(stored) if stored.scope == SCOPE => Kept::Refresh(stored.refresh),
		Some(_) => Kept::Outscoped,
		None if superseded => Kept::Outscoped,
		None => Kept::Nothing,
	}
}

pub(super) fn store(refresh_token: &str) -> miette::Result<()> {
	json::write(
		&path()?,
		&Stored {
			scope: SCOPE.to_owned(),
			refresh: refresh_token.to_owned(),
		},
	)?;

	remove_if_present(&superseded_path()?)
}

fn remove_if_present(path: &Path) -> miette::Result<()> {
	match fs::remove_file(path) {
		Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e)
			.into_diagnostic()
			.wrap_err_with(|| format!("cannot remove {}", path.display())),
		_ => Ok(()),
	}
}

fn path() -> miette::Result<PathBuf> {
	Ok(superseded_path()?.with_extension(json::EXT))
}

fn superseded_path() -> miette::Result<PathBuf> {
	Ok(cache::root()?.join(FILE))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn stored(scope: &str) -> Stored {
		Stored {
			scope: scope.to_owned(),
			refresh: String::from("refresh"),
		}
	}

	#[test]
	fn a_token_granted_the_current_scope_is_refreshed() {
		assert!(
			matches!(kept(Some(stored(SCOPE)), false), Kept::Refresh(refresh) if refresh == "refresh")
		);
	}

	#[test]
	fn a_token_granted_another_scope_is_not_reused() {
		assert!(matches!(
			kept(Some(stored("rating")), false),
			Kept::Outscoped
		));
	}

	#[test]
	fn the_plain_token_written_before_scopes_were_kept_asks_for_a_new_login() {
		assert!(matches!(kept(None, true), Kept::Outscoped));
	}

	#[test]
	fn no_token_at_all_is_nothing_to_refresh() {
		assert!(matches!(kept(None, false), Kept::Nothing));
	}
}
