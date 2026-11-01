use crate::{format, open, prompt};

use super::{Client, SCOPE, token};

pub(super) fn run(client: &Client) -> miette::Result<Option<token::Token>> {
	let login = prompt::confirm("login to musicbrainz", true)?;

	if !login {
		return Ok(None);
	}

	let url = authorize_url(client);
	println!("{}", format::url(&url));
	open::open(&url)?;

	let code = prompt::line("authorization code")?;

	token::exchange(client, &code).map(Some)
}

fn authorize_url(client: &Client) -> String {
	format!(
		"https://musicbrainz.org/oauth2/authorize?response_type=code&client_id={id}&redirect_uri={redirect}&scope={scope}",
		id = client.id,
		redirect = token::REDIRECT_URI.replace(':', "%3A"),
		scope = SCOPE.replace(' ', "+"),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_login_asks_for_every_scope_the_run_submits_under() {
		let client = Client {
			id: String::from("id"),
			secret: String::from("secret"),
		};

		assert!(authorize_url(&client).ends_with("&scope=rating+tag"));
	}
}
