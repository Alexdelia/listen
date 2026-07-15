mod client;
mod login;
mod token;

use ansi::abbrev::{D, Y};

use client::Client;

pub fn acquire() -> hmerr::Result<Option<String>> {
	let Some(client) = Client::from_env() else {
		return Ok(None);
	};

	if let Some(stored) = token::stored()? {
		match token::refresh(&client, &stored) {
			Ok(token) => {
				if let Some(refresh) = &token.refresh {
					token::store(refresh)?;
				}
				return Ok(Some(token.access));
			}
			Err(e) => eprintln!("{Y}stored musicbrainz login no longer works{D}\n{e}"),
		}
	}

	login::run(&client)
}
