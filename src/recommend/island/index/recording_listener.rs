use std::path::Path;

use ansi::abbrev::{D, F};

use super::{
	open::{self, RECORDING_LISTENER, USER_LISTEN},
	progress, query,
};

pub(super) fn counted(holding: &Path) -> String {
	format!(
		r"
select recording_id, count(*)::uinteger as listener, sum(plays)::ubigint as plays
from read_parquet('{listen}/*.parquet')
group by 1
",
		listen = holding.join(USER_LISTEN).display()
	)
}

pub(super) fn derive(dir: &Path) -> hmerr::Result<()> {
	if !open::predates_listener(dir) {
		return Ok(());
	}

	progress::say(format!(
		"{F}index predates the listener count, counted from the listen it holds{D}"
	));

	let db = open::session(dir)?;

	query::copy(&db, &dir.join(RECORDING_LISTENER), &counted(dir))
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use super::*;

	const REPEATED: u32 = 7;
	const BRUSHED: u32 = 9;

	fn holding(name: &str, listen: &[(u32, u32, u32)]) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_listener_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let into = dir.join(USER_LISTEN);
		let _ = fs::create_dir_all(&into);

		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());
		db.execute_batch(&format!(
			"copy (select * from (values {row}) as t(user_id, recording_id, plays)) \
			to '{shard}' (format parquet);",
			row = listen
				.iter()
				.map(|(user, recording, plays)| format!("({user}, {recording}, {plays})"))
				.collect::<Vec<_>>()
				.join(","),
			shard = into.join(open::shard(0)).display()
		))
		.unwrap_or_else(|e| unreachable!("{e}"));

		dir
	}

	fn count(dir: &Path, recording: u32, of: &str) -> i64 {
		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		db.query_row(
			&format!(
				"select {of}::bigint from ({count}) where recording_id = {recording}",
				count = counted(dir)
			),
			[],
			|row| row.get(0),
		)
		.unwrap_or_else(|e| unreachable!("{e}"))
	}

	#[test]
	fn a_listener_playing_a_recording_once_counts_as_much_as_one_repeating_it_forever() {
		let dir = holding(
			"repeat",
			&[(1, REPEATED, 200), (2, REPEATED, 1), (3, BRUSHED, 4)],
		);

		assert_eq!(count(&dir, REPEATED, "listener"), 2);
		assert_eq!(count(&dir, BRUSHED, "listener"), 1);
		assert_eq!(count(&dir, REPEATED, "plays"), 201);
		let _ = fs::remove_dir_all(&dir);
	}
}
