use std::path::{Path, PathBuf};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};
use serde::{Deserialize, Serialize};

use crate::cache;

use super::super::attraction;

pub(super) const DIR: &str = "index";
pub(super) const META: &str = "meta.json";
pub(super) const RECORDING: &str = "recording.parquet";
pub(super) const RECORDING_ARTIST: &str = "recording_artist.parquet";
pub(super) const RECORDING_LISTENER: &str = "recording_listener.parquet";
pub(super) const USER_LISTEN: &str = "user_listen";
pub(super) const USER_STAT: &str = "user_stat.parquet";
pub(super) const ARTIST_LINK: &str = "artist_link.parquet";
pub(super) const BUCKET: u32 = 8;
pub(super) const PLAY_CEILING: u16 = u16::MAX;

const MEMORY_LIMIT: &str = "4GB";

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Gap {
	pub from: String,
	pub to: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Meta {
	pub built: String,
	pub dump: String,
	#[serde(default)]
	pub own: Option<u32>,
	#[serde(default)]
	pub reached: Option<String>,
	#[serde(default)]
	pub gap: Vec<Gap>,
	#[serde(default)]
	pub absorbed: u32,
	pub user: u64,
	pub recording: u64,
	pub user_listen: u64,
}

impl Meta {
	pub(crate) fn covered(&self) -> &str {
		self.reached.as_deref().unwrap_or(&self.dump)
	}
}

pub(crate) struct Index {
	pub db: duckdb::Connection,
	pub meta: Meta,
}

pub(super) fn dir() -> hmerr::Result<PathBuf> {
	Ok(cache::root()?.join(DIR))
}

pub(super) fn shard(bucket: u32) -> String {
	format!("{bucket}.parquet")
}

pub(super) fn indexed(dir: &Path) -> bool {
	scanned(dir) && dir.join(USER_STAT).exists()
}

pub(super) fn predates_stat(dir: &Path) -> bool {
	scanned(dir) && !dir.join(USER_STAT).exists()
}

pub(super) fn predates_listener(dir: &Path) -> bool {
	scanned(dir) && !dir.join(RECORDING_LISTENER).exists()
}

pub(super) fn scanned(dir: &Path) -> bool {
	[RECORDING, RECORDING_ARTIST, META]
		.iter()
		.all(|part| dir.join(part).exists())
		&& bucketed(&dir.join(USER_LISTEN))
}

pub(super) fn bucketed(into: &Path) -> bool {
	(0..BUCKET).all(|bucket| into.join(shard(bucket)).exists())
}

pub(super) fn built(dir: &Path) -> bool {
	indexed(dir) && dir.join(ARTIST_LINK).exists()
}

pub(super) fn session(dir: &Path) -> hmerr::Result<duckdb::Connection> {
	let db = duckdb::Connection::open_in_memory()?;

	db.execute_batch(&format!(
		r"
set memory_limit='{MEMORY_LIMIT}';
set temp_directory='{dir}/spill';
set preserve_insertion_order=false;
",
		dir = dir.display()
	))?;

	Ok(db)
}

pub(super) fn open(dir: &Path) -> hmerr::Result<Index> {
	let meta = meta(dir)?;
	let db = session(dir)?;

	db.execute_batch(&format!(
		r"
create view recording as select * from read_parquet('{dir}/{RECORDING}');
create view recording_artist as select * from read_parquet('{dir}/{RECORDING_ARTIST}');
create view recording_listener as select * from read_parquet('{dir}/{RECORDING_LISTENER}');
create view user_listen as select * from read_parquet('{dir}/{USER_LISTEN}/*.parquet');
create view user_stat as select * from read_parquet('{dir}/{USER_STAT}');
create view artist_link as select * from read_parquet('{dir}/{ARTIST_LINK}');
",
		dir = dir.display()
	))?;

	attraction::declare(&db)?;

	Ok(Index { db, meta })
}

pub(super) fn own(dir: &Path) -> Option<u32> {
	meta(dir).ok().and_then(|meta| meta.own)
}

pub(super) fn forget_meta(dir: &Path) -> hmerr::Result<()> {
	let path = dir.join(META);

	if !path.exists() {
		return Ok(());
	}

	std::fs::remove_file(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

pub(super) fn write_meta(dir: &Path, meta: &Meta) -> hmerr::Result<()> {
	let path = dir.join(META);
	let content = serde_json::to_string(meta)?;

	std::fs::write(&path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

pub(super) fn meta(dir: &Path) -> hmerr::Result<Meta> {
	let path = dir.join(META);
	let content = std::fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	serde_json::from_str(&content).map_err(|e| {
		ge!(
			format!("{R}cannot read {B}{}{D}{R}\n{e}{D}", path.display()),
			h: "delete it to rebuild the index"
		)
		.into()
	})
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::*;

	fn scratch(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_index_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	fn lay_out(dir: &Path, bucket: u32) {
		let into = dir.join(USER_LISTEN);
		let _ = fs::create_dir_all(&into);

		for part in [
			RECORDING,
			RECORDING_ARTIST,
			RECORDING_LISTENER,
			USER_STAT,
			META,
		] {
			let _ = fs::write(dir.join(part), b"built");
		}
		for bucket in 0..bucket {
			let _ = fs::write(into.join(shard(bucket)), b"built");
		}
	}

	#[test]
	fn every_bucket_alongside_the_written_meta_is_an_index() {
		let dir = scratch("whole");
		lay_out(&dir, BUCKET);

		assert!(indexed(&dir));
		assert!(!predates_stat(&dir));
		assert!(!predates_listener(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_written_before_the_listener_stat_is_one_to_recover() {
		let dir = scratch("predate");
		lay_out(&dir, BUCKET);
		let _ = fs::remove_file(dir.join(USER_STAT));

		assert!(!indexed(&dir));
		assert!(predates_stat(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_written_before_the_listener_count_still_counts_as_one() {
		let dir = scratch("uncounted");
		lay_out(&dir, BUCKET);
		let _ = fs::remove_file(dir.join(RECORDING_LISTENER));

		assert!(indexed(&dir));
		assert!(predates_listener(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_stat_cannot_be_recovered_from_listens_that_are_not_all_there() {
		let dir = scratch("half");
		lay_out(&dir, BUCKET / 2);
		let _ = fs::remove_file(dir.join(USER_STAT));

		assert!(!predates_stat(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_build_interrupted_partway_through_the_buckets_is_not_an_index() {
		let dir = scratch("interrupted");
		lay_out(&dir, BUCKET / 2);

		assert!(!indexed(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_whose_meta_is_forgotten_is_built_again() {
		let dir = scratch("forgotten");
		lay_out(&dir, BUCKET);

		assert!(forget_meta(&dir).is_ok());

		assert!(!dir.join(META).exists());
		assert!(!indexed(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn forgetting_a_meta_that_was_never_written_is_not_an_error() {
		let dir = scratch("never");

		assert!(forget_meta(&dir).is_ok());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_without_its_artist_link_is_not_built_yet() {
		let dir = scratch("link");
		lay_out(&dir, BUCKET);

		assert!(!built(&dir));

		let _ = fs::write(dir.join(ARTIST_LINK), b"built");

		assert!(built(&dir));
		let _ = fs::remove_dir_all(&dir);
	}
}
