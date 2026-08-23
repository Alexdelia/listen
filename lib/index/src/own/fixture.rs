use std::{fs, path::PathBuf};

pub(super) const OWN: u32 = 7;
pub(super) const OTHER: u32 = 8;

pub(super) const AAAA: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
pub(super) const BBBB: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

pub(super) struct Listen {
	user: u32,
	mbid: &'static str,
	track: &'static str,
	artist: &'static str,
	at: &'static str,
}

pub(super) fn listen(user: u32, mbid: &'static str, at: &'static str) -> Listen {
	Listen {
		user,
		mbid,
		track: "Fairy Dance",
		artist: "UNDEAD CORPORATION",
		at,
	}
}

pub(super) fn dump(name: &str, listen: &[Listen]) -> PathBuf {
	let dir = std::env::temp_dir().join(format!("declarative_listen_own_{name}"));
	let _ = fs::remove_dir_all(&dir);
	let _ = fs::create_dir_all(&dir);

	let row = listen
		.iter()
		.map(|listen| {
			format!(
				"(timestamp '{at}', {user}, '{mbid}', '{track}', '{artist}')",
				at = listen.at,
				user = listen.user,
				mbid = listen.mbid,
				track = listen.track,
				artist = listen.artist
			)
		})
		.collect::<Vec<_>>()
		.join(",");

	let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());
	db.execute_batch(&format!(
		"copy (select * from (values {row}) \
		as t(listened_at, user_id, recording_mbid, recording_name, artist_name)) \
		to '{shard}' (format parquet);",
		shard = dir.join("0.parquet").display()
	))
	.unwrap_or_else(|e| unreachable!("{e}"));

	dir
}
