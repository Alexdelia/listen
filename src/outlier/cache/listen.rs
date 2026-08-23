use std::path::PathBuf;

use listen_cache::json;

use super::super::fetch::ListenCount;

const SUBDIR: &str = "listen";

pub(in crate::outlier) fn read(username: &str) -> hmerr::Result<Option<ListenCount>> {
	json::read(&path(username)?)
}

pub(in crate::outlier) fn write(username: &str, listen: &ListenCount) -> hmerr::Result<()> {
	json::write(&path(username)?, listen)
}

fn path(username: &str) -> hmerr::Result<PathBuf> {
	listen_cache::path(SUBDIR, username, json::EXT)
}
