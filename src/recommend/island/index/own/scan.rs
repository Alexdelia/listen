use std::path::Path;

use crate::declaration::Source;

use super::super::open::PLAY_CEILING;

pub(crate) struct Play {
	pub mbid: Source,
	pub plays: u32,
	pub track: String,
	pub artist: String,
}

pub(super) struct Scanned {
	pub play: Vec<Play>,
	pub covered: i64,
}

pub(super) fn of(db: &duckdb::Connection, dump: &Path, own: u32) -> hmerr::Result<Scanned> {
	let mut statement = db.prepare(&format!(
		r"
select
	l.recording_mbid,
	arg_max(l.recording_name, l.listened_at),
	arg_max(l.artist_name, l.listened_at),
	least(count(*), {PLAY_CEILING})::uinteger,
	max(epoch(l.listened_at))::bigint
from read_parquet('{dump}/*.parquet') l
where l.user_id = {own} and l.recording_mbid is not null
group by 1
",
		dump = dump.display()
	))?;

	let mut row = statement.query([])?;
	let mut play = Vec::new();
	let mut covered = 0;

	while let Some(row) = row.next()? {
		covered = covered.max(row.get::<_, i64>(4)?);

		let Ok(mbid) = row.get::<_, String>(0)?.parse() else {
			continue;
		};

		play.push(Play {
			mbid,
			plays: row.get(3)?,
			track: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
			artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
		});
	}

	Ok(Scanned { play, covered })
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::{
		super::fixture::{AAAA, BBBB, OTHER, OWN, dump, listen},
		*,
	};

	fn scan(dir: &Path, own: u32) -> Scanned {
		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		of(&db, dir, own).unwrap_or_else(|_| unreachable!())
	}

	#[test]
	fn every_own_listen_of_a_recording_is_one_count_under_its_name() {
		let dir = dump(
			"counted",
			&[
				listen(OWN, AAAA, "2026-07-01 10:00:00"),
				listen(OWN, AAAA, "2026-07-02 10:00:00"),
				listen(OWN, BBBB, "2026-07-03 10:00:00"),
			],
		);

		let scanned = scan(&dir, OWN);
		let played = scanned
			.play
			.iter()
			.map(|play| (play.mbid.to_string(), play.plays))
			.collect::<std::collections::HashMap<String, u32>>();

		assert_eq!(played.get(AAAA), Some(&2));
		assert_eq!(played.get(BBBB), Some(&1));
		assert_eq!(
			scanned.play.first().map(|play| play.track.clone()),
			Some("Fairy Dance".to_string())
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn what_another_listener_played_is_none_of_it() {
		let dir = dump(
			"other",
			&[
				listen(OWN, AAAA, "2026-07-01 10:00:00"),
				listen(OTHER, AAAA, "2026-07-01 10:00:00"),
				listen(OTHER, BBBB, "2026-07-01 10:00:00"),
			],
		);

		let scanned = scan(&dir, OWN);

		assert_eq!(scanned.play.len(), 1);
		assert_eq!(scanned.play.first().map(|play| play.plays), Some(1));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_dump_holding_nothing_of_ours_leaves_us_with_nothing() {
		let dir = dump("absent", &[listen(OTHER, AAAA, "2026-07-01 10:00:00")]);

		assert!(scan(&dir, OWN).play.is_empty());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn how_far_the_dump_covers_is_the_last_listen_it_holds_of_ours() {
		let dir = dump(
			"covered",
			&[
				listen(OWN, AAAA, "2026-07-01 10:00:00"),
				listen(OWN, BBBB, "2026-07-11 20:39:04"),
				listen(OTHER, AAAA, "2026-07-12 00:00:00"),
			],
		);

		let last = chrono::DateTime::from_timestamp(scan(&dir, OWN).covered, 0)
			.map(|last| last.format("%Y-%m-%d %H:%M:%S").to_string());

		assert_eq!(last, Some("2026-07-11 20:39:04".to_string()));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_listen_mapped_to_nothing_readable_is_skipped() {
		let dir = dump(
			"unreadable",
			&[
				listen(OWN, "not-an-mbid", "2026-07-01 10:00:00"),
				listen(OWN, AAAA, "2026-07-01 10:00:00"),
			],
		);

		let scanned = scan(&dir, OWN);

		assert_eq!(scanned.play.len(), 1);
		assert_eq!(
			scanned.play.first().map(|play| play.mbid.to_string()),
			Some(AAAA.to_string())
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_listen_mapped_to_nothing_readable_is_still_a_listen_the_dump_holds() {
		let dir = dump(
			"unreadable_last",
			&[
				listen(OWN, AAAA, "2026-07-01 10:00:00"),
				listen(OWN, "not-an-mbid", "2026-07-11 20:39:04"),
			],
		);

		let last = chrono::DateTime::from_timestamp(scan(&dir, OWN).covered, 0)
			.map(|last| last.format("%Y-%m-%d %H:%M:%S").to_string());

		assert_eq!(last, Some("2026-07-11 20:39:04".to_string()));
		let _ = fs::remove_dir_all(&dir);
	}
}
