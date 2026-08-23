use ansi::abbrev::{B, D};

use crate::{open, prompt};

use super::{Client, token};

const SCOPE: &str = "rating";

pub(super) fn run(client: &Client) -> hmerr::Result<Option<String>> {
	let login = prompt::confirm("login to musicbrainz", true)?;

	if !login {
		return Ok(None);
	}

	let url = authorize_url(client);
	println!("{B}{url}{D}");
	open::open(&url)?;

	let code = prompt::line("authorization code")?;

	let token = token::exchange(client, &code)?;
	if let Some(refresh) = &token.refresh {
		token::store(refresh)?;
	}

	Ok(Some(token.access))
}

fn authorize_url(client: &Client) -> String {
	format!(
		"https://musicbrainz.org/oauth2/authorize?response_type=code&client_id={id}&redirect_uri={redirect}&scope={SCOPE}",
		id = client.id,
		redirect = token::REDIRECT_URI.replace(':', "%3A"),
	)
}
