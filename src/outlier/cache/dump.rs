use std::path::PathBuf;

use listen_cache::json;

use super::super::dump::Held;

const SUBDIR: &str = "own";

pub(in crate::outlier) fn read(username: &str) -> hmerr::Result<Option<Held>> {
	json::read(&path(username)?)
}

pub(in crate::outlier) fn write(username: &str, held: &Held) -> hmerr::Result<()> {
	json::write(&path(username)?, held)
}

fn path(username: &str) -> hmerr::Result<PathBuf> {
	listen_cache::path(SUBDIR, username, json::EXT)
}
