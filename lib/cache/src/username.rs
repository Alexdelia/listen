use std::path::PathBuf;

use console::style;
use listen_prompt::{confirm, line};

use crate::{root, text};

const FILE: &str = "username";

pub fn resolve(username: Option<&str>) -> miette::Result<String> {
	if let Some(username) = username {
		remember(username)?;
		return Ok(username.to_string());
	}

	if let Some(cached) = read()? {
		return Ok(cached);
	}

	let username = line("listenbrainz username")?;
	store(&username)?;

	Ok(username)
}

fn path() -> miette::Result<PathBuf> {
	Ok(root()?.join(FILE))
}

pub fn read() -> miette::Result<Option<String>> {
	text::read(&path()?)
}

fn remember(username: &str) -> miette::Result<()> {
	let Some(remembered) = read()? else {
		return store(username);
	};

	if remembered == username || !instead(&remembered, username)? {
		return Ok(());
	}

	store(username)
}

fn instead(remembered: &str, username: &str) -> miette::Result<bool> {
	println!(
		"\n{}{}",
		style(remembered).bold(),
		style(
			" is the name remembered, \
		which is the listener an index build calls its own \
		and leaves out of what it recommends"
		)
		.dim()
	);

	confirm(
		&format!("remember {} instead", style(username).bold()),
		false,
	)
}

fn store(username: &str) -> miette::Result<()> {
	text::write(&path()?, username)
}
