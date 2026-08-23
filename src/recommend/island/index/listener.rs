use ansi::abbrev::{B, D, F, G, Y};

use crate::cache::listener::{self, Named};

use super::{dump, progress};

pub(super) fn of(username: &str) -> hmerr::Result<Option<u32>> {
	if cfg!(test) {
		return Ok(None);
	}

	if let Some(named) = listener::read(username)? {
		return Ok(named.id);
	}

	looking(username);

	let id = match dump::named(username) {
		Ok(id) => id,
		Err(e) => {
			nameless(username, &e.to_string());
			return Ok(None);
		}
	};

	listener::write(username, &Named { id })?;
	told(username, id);

	Ok(id)
}

fn looking(username: &str) {
	progress::say(format!(
		"{F}looking {B}{username}{D}{F} up in the listens dump, \
		which is what ties a name to the number its listens are dumped under, \
		once per name{D}"
	));
}

fn told(username: &str, id: Option<u32>) {
	match id {
		Some(id) => progress::say(format!("{B}{username}{D} listens as {B}{G}{id}{D}")),
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
