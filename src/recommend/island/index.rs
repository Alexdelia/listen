use std::path::Path;

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};
use serde::Deserialize;

use crate::cache;

const DIR: &str = "index";
const META: &str = "meta.json";
const BUILDER: &str = "research/index/build.sh";

const MEMORY_LIMIT: &str = "4GB";

#[derive(Deserialize)]
pub(super) struct Meta {
	pub built: String,
	pub user: u64,
	pub recording: u64,
	pub user_listen: u64,
}

pub(super) struct Index {
	pub db: duckdb::Connection,
	pub meta: Meta,
}

pub(super) fn open() -> hmerr::Result<Index> {
	let dir = cache::root()?.join(DIR);
	let meta = meta(&dir)?;
	let db = duckdb::Connection::open_in_memory()?;

	db.execute_batch(&format!(
		"
set memory_limit='{MEMORY_LIMIT}';
set temp_directory='{dir}/spill';
set preserve_insertion_order=false;
create view recording as select * from read_parquet('{dir}/recording.parquet');
create view recording_artist as select * from read_parquet('{dir}/recording_artist.parquet');
create view user_listen as select * from read_parquet('{dir}/user_listen/*.parquet');
create view artist_link as select * from read_parquet('{dir}/artist_link.parquet');
",
		dir = dir.display()
	))?;

	Ok(Index { db, meta })
}

fn meta(dir: &Path) -> hmerr::Result<Meta> {
	let path = dir.join(META);

	if !path.exists() {
		return Err(missing(dir).into());
	}

	let content = std::fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	serde_json::from_str(&content).map_err(|e| {
		ge!(format!(
			"{R}cannot read {B}{}{D}{R}\n{e}{D}",
			path.display()
		))
		.into()
	})
}

fn missing(dir: &Path) -> hmerr::GenericError {
	ge!(
		format!("{R}no listen index at {B}{}{D}", dir.display()),
		h: format!("build it with {B}{BUILDER}{D}")
	)
}
