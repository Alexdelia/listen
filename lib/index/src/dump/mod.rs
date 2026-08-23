mod incremental;
mod listen;
mod listener;
mod music_brainz;
mod rsync;
mod space;
mod stage;
mod stamp;

use std::path::{Path, PathBuf};

use ansi::abbrev::{D, F};
use hmerr::ioe;
use indicatif::ProgressBar;

use listen_cache as cache;

use crate::{decide::Decide, progress};

pub(super) use incremental::{Incremental, Pending};
pub(super) use listen::Listen;
pub(super) use listener::Search;

use listen::Offer;

const DIR: &str = "dump";

const NO_INDEX: Offer = Offer {
	reason: "no index yet, the whole dump is read once to build one",
	default: true,
};

const HOLED: Offer = Offer {
	reason: "the index has a hole no incremental can fill, only a full dump repairs it",
	default: false,
};

pub(super) fn root() -> hmerr::Result<PathBuf> {
	let root = cache::root()?.join(DIR);
	std::fs::create_dir_all(&root).map_err(|e| ioe!(root.to_string_lossy(), e))?;

	Ok(root)
}

pub(super) fn listen(decide: &dyn Decide) -> hmerr::Result<Listen> {
	let root = root()?;

	match listen::find(&root)? {
		Some(listen) => Ok(listen),
		None => listen::fetch(&root, &NO_INDEX, decide)?.ok_or_else(|| listen::refused().into()),
	}
}

pub(super) fn named(username: &str, past: Option<u64>) -> hmerr::Result<Search> {
	listener::named(username, past)
}

pub(super) fn unpacked() -> hmerr::Result<Option<Listen>> {
	listen::find(&root()?)
}

pub(super) fn discard(listen: &Listen) -> hmerr::Result<()> {
	listen::discard(listen)
}

pub(super) fn repairable(baseline: &str) -> hmerr::Result<Option<String>> {
	let root = root()?;

	let Some(dump) = listen::newer_than(baseline)? else {
		return Ok(None);
	};

	if listen::declined(&root).as_deref() == Some(dump.as_str()) {
		return Ok(None);
	}

	Ok(Some(dump))
}

pub(super) fn repair(dump: &str, decide: &dyn Decide) -> hmerr::Result<Option<Listen>> {
	let root = root()?;
	let taken = listen::fetch_named(&root, dump, &HOLED, decide)?;

	if taken.is_none() {
		listen::decline(&root, dump)?;
	}

	Ok(taken)
}

pub(super) fn pending(covered: &str) -> hmerr::Result<Vec<Pending>> {
	incremental::pending(covered)
}

pub(super) fn listed<T>(read: hmerr::Result<T>, keeping: &str) -> Option<T> {
	match read {
		Ok(read) => Some(read),
		Err(e) => {
			progress::say(format!(
				"{F}cannot read what is published, keeping {keeping}{D}\n{e}"
			));

			None
		}
	}
}

pub(super) fn reach(timestamp: &str) -> hmerr::Result<u64> {
	stamp::reach(timestamp)
}

pub(super) fn weight(pending: &[&Pending]) -> u64 {
	incremental::weight(pending)
}

pub(super) fn room(root: &Path, pending: &[&Pending], at_once: u64) -> hmerr::Result<()> {
	incremental::room(root, pending, at_once)
}

pub(super) fn pull(
	root: &Path,
	pending: &Pending,
	downloading: &ProgressBar,
	verifying: &ProgressBar,
) -> hmerr::Result<()> {
	incremental::pull(root, pending, downloading, verifying)
}

pub(super) fn opened(
	root: &Path,
	pending: &Pending,
	bar: &ProgressBar,
) -> hmerr::Result<Incremental> {
	incremental::opened(root, pending, bar)
}

pub(super) fn release(incremental: &Incremental) -> hmerr::Result<()> {
	incremental::release(incremental)
}

pub(super) fn artist_link(link: &Path, decide: &dyn Decide) -> hmerr::Result<()> {
	if link.exists() {
		return Ok(());
	}

	music_brainz::build(&root()?, link, decide)
}

#[cfg(test)]
mod tests {
	use hmerr::ge;

	use super::*;

	const KEEPING: &str = "what stands as it stands";

	fn read(pending: hmerr::Result<Vec<Pending>>) -> Option<Vec<Pending>> {
		listed(pending, KEEPING)
	}

	#[test]
	fn nothing_published_can_be_read_leaves_what_stands_where_it_is() {
		assert!(read(Err(ge!("rsync is not here".to_string()).into())).is_none());
	}

	#[test]
	fn what_is_published_is_read_as_it_comes() {
		assert!(read(Ok(Vec::new())).is_some_and(|pending| pending.is_empty()));
	}
}
