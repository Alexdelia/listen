#[cfg(test)]
mod fixture;
mod fold;
mod held;
mod kept;
mod say;
mod scan;

use crate::recommend::island::index::own;

use super::cache;

pub(super) use held::Held;

use kept::{Kept, kept, told};

pub(super) fn listen(username: &str, refresh: bool) -> hmerr::Result<Option<Held>> {
	let Some(mut held) = held(username, refresh)? else {
		return Ok(None);
	};

	fold::folded(username, &mut held)?;
	say::announce(username, &held)?;

	Ok(Some(held))
}

fn held(username: &str, refresh: bool) -> hmerr::Result<Option<Held>> {
	let unpacked = own::unpacked()?;
	let cached = cache::dump::read(username)?;

	match kept(cached, unpacked.as_deref(), refresh) {
		Kept::Cached(held) => Ok(Some(held)),
		Kept::Rescan(_) if unpacked.is_none() => Ok(None),
		Kept::Rescan(rescan) => scan::scanned(username, told(rescan)),
	}
}
