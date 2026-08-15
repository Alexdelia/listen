mod client;
mod login;
mod token;

use ansi::abbrev::{D, Y};

pub(in crate::sync::rate) use client::Client;

pub(super) fn client() -> Option<Client> {
	Client::from_env()
}

pub(super) fn acquire(client: &Client) -> hmerr::Result<Option<String>> {
	if let Some(stored) = token::stored()? {
		match token::refresh(client, &stored)? {
			token::Refresh::Token(token) => {
				if let Some(refresh) = &token.refresh {
					token::store(refresh)?;
				}
				return Ok(Some(token.access));
			}
			token::Refresh::Rejected => {
				eprintln!("{Y}musicbrainz login expired, logging in again{D}");
			}
		}
	}

	login::run(client)
}
