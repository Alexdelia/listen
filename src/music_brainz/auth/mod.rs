mod client;
mod held;
mod login;
mod token;

use console::style;

pub(crate) use client::Client;

const SCOPE: &str = "rating tag";

pub(crate) fn client() -> Option<Client> {
	Client::from_env()
}

pub(crate) fn acquire(client: &Client) -> miette::Result<Option<String>> {
	if let Some(access) = held::access() {
		return Ok(Some(access));
	}

	let Some(token) = obtained(client)? else {
		return Ok(None);
	};

	if let Some(refresh) = &token.refresh {
		token::store(refresh)?;
	}
	held::keep(&token);

	Ok(Some(token.access))
}

fn obtained(client: &Client) -> miette::Result<Option<token::Token>> {
	match token::stored()? {
		token::Kept::Refresh(stored) => match token::refresh(client, &stored)? {
			token::Refresh::Token(token) => return Ok(Some(token)),
			token::Refresh::Rejected => eprintln!(
				"{}",
				style("musicbrainz login expired, logging in again").yellow()
			),
		},
		token::Kept::Outscoped => eprintln!(
			"{}",
			style(format!(
				"musicbrainz login predates the \"{SCOPE}\" scope, logging in again"
			))
			.yellow()
		),
		token::Kept::Nothing => {}
	}

	login::run(client)
}
