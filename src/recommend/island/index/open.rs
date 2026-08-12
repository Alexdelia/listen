use std::path::{Path, PathBuf};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};
use serde::{Deserialize, Serialize};

use crate::cache;

pub(super) const DIR: &str = "index";
pub(super) const META: &str = "meta.json";
pub(super) const RECORDING: &str = "recording.parquet";
pub(super) const RECORDING_ARTIST: &str = "recording_artist.parquet";
pub(super) const USER_LISTEN: &str = "user_listen";
pub(super) const ARTIST_LINK: &str = "artist_link.parquet";

const MEMORY_LIMIT: &str = "4GB";

#[derive(Deserialize, Serialize)]
pub(crate) struct Meta {
	pub built: String,
	pub dump: String,
	pub seed: u64,
	pub user: u64,
	pub recording: u64,
	pub user_listen: u64,
}

pub(crate) struct Index {
	pub db: duckdb::Connection,
	pub meta: Meta,
}

pub(super) fn dir() -> hmerr::Result<PathBuf> {
	Ok(cache::root()?.join(DIR))
}

pub(super) fn indexed(dir: &Path) -> bool {
	[RECORDING, RECORDING_ARTIST, META]
		.iter()
		.all(|part| dir.join(part).exists())
		&& dir.join(USER_LISTEN).is_dir()
}

pub(super) fn built(dir: &Path) -> bool {
	indexed(dir) && dir.join(ARTIST_LINK).exists()
}

pub(super) fn open(dir: &Path) -> hmerr::Result<Index> {
	let meta = meta(dir)?;
	let db = duckdb::Connection::open_in_memory()?;

	db.execute_batch(&format!(
		r"
set memory_limit='{MEMORY_LIMIT}';
set temp_directory='{dir}/spill';
set preserve_insertion_order=false;
create view recording as select * from read_parquet('{dir}/{RECORDING}');
create view recording_artist as select * from read_parquet('{dir}/{RECORDING_ARTIST}');
create view user_listen as select * from read_parquet('{dir}/{USER_LISTEN}/*.parquet');
create view artist_link as select * from read_parquet('{dir}/{ARTIST_LINK}');
",
		dir = dir.display()
	))?;

	Ok(Index { db, meta })
}

pub(super) fn write_meta(dir: &Path, meta: &Meta) -> hmerr::Result<()> {
	let path = dir.join(META);
	let content = serde_json::to_string(meta)?;

	std::fs::write(&path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn meta(dir: &Path) -> hmerr::Result<Meta> {
	let path = dir.join(META);
	let content = std::fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	serde_json::from_str(&content).map_err(|e| {
		ge!(
			format!("{R}cannot read {B}{}{D}{R}\n{e}{D}", path.display()),
			h: "delete it to have the index rebuilt"
		)
		.into()
	})
}
