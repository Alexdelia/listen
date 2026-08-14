mod listen;
mod music_brainz;
mod rsync;
mod space;

use std::path::{Path, PathBuf};

use hmerr::ioe;

use crate::cache;

pub(super) use listen::Listen;

const DIR: &str = "dump";

pub(super) fn root() -> hmerr::Result<PathBuf> {
	let root = cache::root()?.join(DIR);
	std::fs::create_dir_all(&root).map_err(|e| ioe!(root.to_string_lossy(), e))?;

	Ok(root)
}

pub(super) fn listen() -> hmerr::Result<Listen> {
	let root = root()?;

	match listen::find(&root)? {
		Some(listen) => Ok(listen),
		None => listen::fetch(&root),
	}
}

pub(super) fn discard(listen: &Listen) -> hmerr::Result<()> {
	listen::discard(listen)
}

pub(super) fn artist_link(link: &Path) -> hmerr::Result<()> {
	if link.exists() {
		return Ok(());
	}

	music_brainz::build(&root()?, link)
}
