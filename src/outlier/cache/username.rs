use std::{
	fs,
	io::{self, Write},
	path::PathBuf,
};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

const FILE: &str = "username";

pub(in crate::outlier) fn resolve(username: Option<&str>) -> hmerr::Result<String> {
	if let Some(username) = username {
		store(username)?;
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
	Ok(super::root()?.join(FILE))
}

fn read() -> hmerr::Result<Option<String>> {
	let path = path()?;

	if !path.exists() {
		return Ok(None);
	}

	let content = fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	let username = content.trim();

	Ok((!username.is_empty()).then(|| username.to_string()))
}

fn store(username: &str) -> hmerr::Result<()> {
	let path = path()?;
	super::prepare(&path)?;

	fs::write(&path, username).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
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
