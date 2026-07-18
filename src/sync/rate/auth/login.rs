use std::io::{self, Write};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

use crate::open;

use super::{Client, token};

const SCOPE: &str = "rating";

pub(super) fn run(client: &Client) -> hmerr::Result<Option<String>> {
	let login =
		ux::ask_yn("login to musicbrainz to sync rating?", true).map_err(|e| ioe!("stdin", e))?;

	if !login {
		return Ok(None);
	}

	let url = authorize_url(client);
	println!("{B}{url}{D}");
	open::open(&url)?;

	let code = prompt_code()?;

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

fn prompt_code() -> hmerr::Result<String> {
	print!("{B}authorization code{D}: ");
	io::stdout().flush().map_err(|e| ioe!("stdout", e))?;

	let mut line = String::new();
	io::stdin()
		.read_line(&mut line)
		.map_err(|e| ioe!("stdin", e))?;

	let code = line.trim().to_string();
	if code.is_empty() {
		return Err(ge!(format!("{R}no {B}authorization code{D} provided")).into());
	}

	Ok(code)
}
