use std::{
	io::{self, Write},
	path::PathBuf,
};

use ansi::abbrev::{B, D, F, R};
use hmerr::{ge, ioe};

use crate::{root, text};

const FILE: &str = "username";

pub fn resolve(username: Option<&str>) -> hmerr::Result<String> {
	if let Some(username) = username {
		remember(username)?;
		return Ok(username.to_string());
	}

	if let Some(cached) = read()? {
		return Ok(cached);
	}

	let username = prompt()?;
	store(&username)?;

	Ok(username)
}

fn path() -> hmerr::Result<PathBuf> {
	Ok(root()?.join(FILE))
}

pub fn read() -> hmerr::Result<Option<String>> {
	text::read(&path()?)
}

fn remember(username: &str) -> hmerr::Result<()> {
	let Some(remembered) = read()? else {
		return store(username);
	};

	if remembered == username || !instead(&remembered, username)? {
		return Ok(());
	}

	store(username)
}

fn instead(remembered: &str, username: &str) -> hmerr::Result<bool> {
	println!(
		"\n{B}{remembered}{D}{F} is the name remembered, \
		which is the listener an index build calls its own \
		and leaves out of what it recommends{D}"
	);

	ux::ask_yn(&format!("remember {B}{username}{D} instead"), false)
		.map_err(|e| ioe!("stdin", e).into())
}

fn store(username: &str) -> hmerr::Result<()> {
	text::write(&path()?, username)
}

fn prompt() -> hmerr::Result<String> {
	print!("{B}listenbrainz username{D}: ");
	io::stdout().flush().map_err(|e| ioe!("stdout", e))?;

	let mut line = String::new();
	io::stdin()
		.read_line(&mut line)
		.map_err(|e| ioe!("stdin", e))?;

	let username = line.trim().to_string();
	if username.is_empty() {
		return Err(ge!(format!("{R}no {B}username{D} provided")).into());
	}

	Ok(username)
}
