use console::style;

use crate::{open, prompt};

use super::{Client, token};

pub(super) const SCOPE: &str = "rating tag";

pub(super) fn run(client: &Client) -> miette::Result<Option<String>> {
	let login = prompt::confirm("login to musicbrainz", true)?;

	if !login {
		return Ok(None);
	}

	let url = authorize_url(client);
	println!("{}", style(&url).bold());
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
		"https://musicbrainz.org/oauth2/authorize?response_type=code&client_id={id}&redirect_uri={redirect}&scope={scope}",
		id = client.id,
		redirect = token::REDIRECT_URI.replace(':', "%3A"),
		scope = SCOPE.replace(' ', "+"),
	)
}
