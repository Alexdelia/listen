mod cache;

use cache::Named;
use console::style;

use super::{
	dump::{self, Search},
	progress,
};

pub(super) fn of(username: &str) -> miette::Result<Option<u32>> {
	if cfg!(test) {
		return Ok(None);
	}

	let read = cache::read(username)?;

	if let Some(Named { id: Some(id), .. }) = read {
		return Ok(Some(id));
	}

	let past = read.and_then(|named| named.reach);

	looking(username);

	let search = match dump::named(username, past) {
		Ok(search) => search,
		Err(e) => {
			nameless(username, &format!("{e:?}"));
			return Ok(None);
		}
	};

	cache::write(
		username,
		&Named {
			id: search.id,
			reach: search.reach,
		},
	)?;
	told(username, &search, past);

	Ok(search.id)
}

fn looking(username: &str) {
	progress::say(format!(
		"{}{}{}",
		style("looking ").dim(),
		style(username).dim().bold(),
		style(
			" up in the listens dump, \
		which is what ties a name to the number its listens are dumped under, \
		once per name"
		)
		.dim()
	));
}

fn told(username: &str, search: &Search, past: Option<u64>) {
	match search.id {
		Some(id) => progress::say(format!(
			"{} listens as {}",
			style(username).bold(),
			style(id).bold().green()
		)),
		None if search.reach == past => progress::say(format!(
			"{}{}{}",
			style(
				"no listens dump has been published since the one \
			"
			)
			.yellow(),
			style(username).bold().yellow(),
			style(
				" was already looked for in, \
			the listener the declaration singles out is used instead"
			)
			.yellow()
		)),
		None => progress::say(format!(
			"{}{}{}",
			style("nothing was listened to under ").yellow(),
			style(username).bold().yellow(),
			style(
				" over the dump that was read, \
			the listener the declaration singles out is used instead"
			)
			.yellow()
		)),
	}
}

fn nameless(username: &str, reason: &str) {
	progress::say(format!(
		"{}{}{}\n{reason}",
		style("cannot look ").yellow(),
		style(username).bold().yellow(),
		style(
			" up, \
		the listener the declaration singles out is used instead"
		)
		.yellow()
	));
}
