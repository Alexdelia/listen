use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use listen_cache::json;

const SUBDIR: &str = "listener";

#[derive(Deserialize, Serialize)]
pub(crate) struct Named {
	pub id: Option<u32>,
	#[serde(default)]
	pub reach: Option<u64>,
}

pub(crate) fn read(username: &str) -> hmerr::Result<Option<Named>> {
	json::read(&path(username)?)
}

pub(crate) fn write(username: &str, named: &Named) -> hmerr::Result<()> {
	json::write(&path(username)?, named)
}

fn path(username: &str) -> hmerr::Result<PathBuf> {
	listen_cache::path(SUBDIR, username, json::EXT)
}
