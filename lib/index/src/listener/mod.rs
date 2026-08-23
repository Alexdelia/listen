mod cache;

use ansi::abbrev::{B, D, F, G, Y};

use cache::Named;

use super::{
	dump::{self, Search},
	progress,
};

pub(super) fn of(username: &str) -> hmerr::Result<Option<u32>> {
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
			nameless(username, &e.to_string());
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
		"{F}looking {B}{username}{D}{F} up in the listens dump, \
		which is what ties a name to the number its listens are dumped under, \
		once per name{D}"
	));
}

fn told(username: &str, search: &Search, past: Option<u64>) {
	match search.id {
		Some(id) => progress::say(format!("{B}{username}{D} listens as {B}{G}{id}{D}")),
		None if search.reach == past => progress::say(format!(
			"{Y}no listens dump has been published since the one \
			{B}{username}{D}{Y} was already looked for in, \
			the listener the declaration singles out is used instead{D}"
		)),
		None => progress::say(format!(
			"{Y}nothing was listened to under {B}{username}{D}{Y} over the dump that was read, \
			the listener the declaration singles out is used instead{D}"
		)),
	}
}

fn nameless(username: &str, reason: &str) {
	progress::say(format!(
		"{Y}cannot look {B}{username}{D}{Y} up, \
		the listener the declaration singles out is used instead{D}\n{reason}"
	));
}
