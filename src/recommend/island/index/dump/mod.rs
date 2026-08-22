mod board;
mod incremental;
mod listen;
mod music_brainz;
mod rsync;
mod space;
mod stamp;

use std::path::{Path, PathBuf};

use hmerr::ioe;
use indicatif::ProgressBar;

use crate::cache;

pub(super) use incremental::{Incremental, Pending};
pub(super) use listen::Listen;

use listen::Offer;

const DIR: &str = "dump";

const NO_INDEX: Offer = Offer {
	reason: "no index yet, the whole dump is read once to build one",
	enter_is: true,
};

const HOLED: Offer = Offer {
	reason: "the index has a hole no incremental can fill, only a full dump repairs it",
	enter_is: false,
};

pub(super) fn root() -> hmerr::Result<PathBuf> {
	let root = cache::root()?.join(DIR);
	std::fs::create_dir_all(&root).map_err(|e| ioe!(root.to_string_lossy(), e))?;

	Ok(root)
}

pub(super) fn listen() -> hmerr::Result<Listen> {
	let root = root()?;

	match listen::find(&root)? {
		Some(listen) => Ok(listen),
		None => listen::fetch(&root, &NO_INDEX)?.ok_or_else(|| listen::refused().into()),
	}
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

pub(super) fn repair(dump: &str) -> hmerr::Result<Option<Listen>> {
	let root = root()?;
	let taken = listen::fetch_named(&root, dump, &HOLED)?;

	if taken.is_none() {
		listen::decline(&root, dump)?;
	}

	Ok(taken)
}

pub(super) fn pending(covered: &str) -> hmerr::Result<Vec<Pending>> {
	incremental::pending(covered)
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

pub(super) fn artist_link(link: &Path) -> hmerr::Result<()> {
	if link.exists() {
		return Ok(());
	}

	music_brainz::build(&root()?, link)
}
