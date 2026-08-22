use std::path::Path;

use ansi::abbrev::{B, D, F, Y};

use crate::declaration::Source;

use super::{
	board::Board,
	dump::{self, Incremental, Pending},
	open::{self, PLAY_CEILING},
	progress::{self, Measure},
};

const AT_ONCE: u64 = 2;

const DOWNLOAD: &str = "download";
const VERIFY: &str = "verify";
const UNPACK: &str = "unpack";
const LISTEN: &str = "listen";

pub(crate) struct Play {
	pub mbid: Source,
	pub plays: u32,
	pub track: String,
	pub artist: String,
}

pub(crate) struct Own {
	pub dump: String,
	pub covered: i64,
	pub play: Vec<Play>,
}

pub(crate) struct Fresh {
	pub reached: String,
	pub covered: i64,
	pub play: Vec<Play>,
}

struct Scanned {
	play: Vec<Play>,
	covered: i64,
}

pub(crate) fn unpacked() -> hmerr::Result<Option<String>> {
	Ok(dump::unpacked()?.map(|listen| listen.name))
}

pub(crate) fn played() -> hmerr::Result<Option<Own>> {
	let dir = open::dir()?;

	let Some(own) = open::own(&dir) else {
		return Ok(None);
	};
	let Some(listen) = dump::unpacked()? else {
		return Ok(None);
	};

	let scanned = scanned(&open::session(&dir)?, &listen.dir, own)?;

	if scanned.play.is_empty() {
		return Ok(None);
	}

	Ok(Some(Own {
		dump: listen.name,
		covered: scanned.covered,
		play: scanned.play,
	}))
}

pub(crate) fn fresh(reached: &str) -> hmerr::Result<Option<Fresh>> {
	let dir = open::dir()?;

	let Some(own) = open::own(&dir) else {
		return Ok(None);
	};

	let pending = dump::pending(reached)?;
	let pending: Vec<&Pending> = pending.iter().collect();

	if pending.is_empty() || !offered(&pending)? {
		return Ok(None);
	}

	let root = dump::root()?;
	dump::room(&root, &pending, AT_ONCE)?;

	folded(&open::session(&dir)?, &root, &pending, own, reached).map(Some)
}

fn offered(pending: &[&Pending]) -> hmerr::Result<bool> {
	progress::say(format!(
		"\n{F}{B}{count}{D}{F} incremental dump published since those counts were read, \
		{B}{Y}{size}{D}{F}, each read once then deleted{D}",
		count = pending.len(),
		size = progress::bytes(dump::weight(pending))
	));

	progress::ask("download", true)
}

fn folded(
	db: &duckdb::Connection,
	root: &Path,
	pending: &[&Pending],
	own: u32,
	reached: &str,
) -> hmerr::Result<Fresh> {
	let board = Board::of(&[
		(DOWNLOAD, Measure::Byte(dump::weight(pending))),
		(VERIFY, Measure::Step(step(pending))),
		(UNPACK, Measure::Byte(dump::weight(pending))),
		(LISTEN, Measure::Step(step(pending))),
	])?;

	let downloading = board.start(DOWNLOAD)?;
	let verifying = board.start(VERIFY)?;
	let unpacking = board.start(UNPACK)?;
	let reading = board.start(LISTEN)?;

	let mut fresh = Fresh {
		reached: reached.to_string(),
		covered: 0,
		play: Vec::new(),
	};

	for pending in pending {
		dump::pull(root, pending, &downloading, &verifying)?;
		let incremental = dump::opened(root, pending, &unpacking)?;

		taken(db, &incremental, own, &mut fresh)?;
		reading.inc(1);

		dump::release(&incremental)?;
	}

	Ok(fresh)
}

fn taken(
	db: &duckdb::Connection,
	incremental: &Incremental,
	own: u32,
	fresh: &mut Fresh,
) -> hmerr::Result<()> {
	if behind(&fresh.reached, &incremental.start) {
		skipped(incremental, fresh);
		return Ok(());
	}

	if lost(&fresh.reached, &incremental.start) {
		out_of_reach(&fresh.reached, &incremental.start);
	}

	let scanned = scanned(db, &incremental.dir, own)?;
	fresh.play.extend(scanned.play);
	fresh.covered = fresh.covered.max(scanned.covered);
	fresh.reached.clone_from(&incremental.end);

	Ok(())
}

fn skipped(incremental: &Incremental, fresh: &mut Fresh) {
	already_counted(&incremental.name);

	if !lost(&fresh.reached, &incremental.end) {
		return;
	}

	stays_out(&fresh.reached, &incremental.end);
	fresh.reached.clone_from(&incremental.end);
}

fn step(pending: &[&Pending]) -> u64 {
	u64::try_from(pending.len()).unwrap_or_default()
}

fn lost(reached: &str, start: &str) -> bool {
	dump::reach(start)
		.ok()
		.zip(dump::reach(reached).ok())
		.is_some_and(|(start, reached)| start > reached)
}

fn behind(reached: &str, start: &str) -> bool {
	dump::reach(start)
		.ok()
		.zip(dump::reach(reached).ok())
		.is_some_and(|(start, reached)| start < reached)
}

fn out_of_reach(reached: &str, start: &str) {
	progress::say(format!(
		"{Y}nothing published covers {B}{reached}{D}{Y} to {B}{start}{D}{Y}, \
		those listens stay out of the count{D}"
	));
}

fn already_counted(name: &str) {
	progress::say(format!(
		"{Y}{B}{name}{D}{Y} reaches back into what the count already holds, \
		skipped rather than counted twice{D}"
	));
}

fn stays_out(reached: &str, end: &str) {
	progress::say(format!(
		"{Y}{B}{reached}{D}{Y} to {B}{end}{D}{Y} stays out of the count with it{D}"
	));
}

fn scanned(db: &duckdb::Connection, dump: &Path, own: u32) -> hmerr::Result<Scanned> {
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
		let Ok(mbid) = row.get::<_, String>(0)?.parse() else {
			continue;
		};

		play.push(Play {
			mbid,
			plays: row.get(3)?,
			track: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
			artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
		});
		covered = covered.max(row.get::<_, i64>(4)?);
	}

	Ok(Scanned { play, covered })
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use super::*;

	const OWN: u32 = 7;
	const OTHER: u32 = 8;

	struct Listen {
		user: u32,
		mbid: &'static str,
		track: &'static str,
		artist: &'static str,
		at: &'static str,
	}

	const AAAA: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
	const BBBB: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

	fn listen(user: u32, mbid: &'static str, at: &'static str) -> Listen {
		Listen {
			user,
			mbid,
			track: "Fairy Dance",
			artist: "UNDEAD CORPORATION",
			at,
		}
	}

	fn dump(name: &str, listen: &[Listen]) -> PathBuf {
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

	fn scan(dir: &Path, own: u32) -> Scanned {
		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		scanned(&db, dir, own).unwrap_or_else(|_| unreachable!())
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
	fn a_dump_starting_where_the_count_stopped_leaves_nothing_out_of_reach() {
		assert!(!lost(
			"2026-08-21 00:00:03.155180+00:00",
			"2026-08-21 00:00:03.155180+00:00"
		));
		assert!(!lost(
			"2026-08-22 00:00:02.641933+00:00",
			"2026-08-21 00:00:03.155180+00:00"
		));
	}

	#[test]
	fn a_dump_starting_past_where_the_count_stopped_leaves_a_window_out_of_reach() {
		assert!(lost(
			"2026-07-12 00:00:04.001868+00:00",
			"2026-07-23 00:00:03.690928+00:00"
		));
	}

	fn incremental(name: &str, start: &str, end: &str, dir: PathBuf) -> Incremental {
		Incremental {
			dir,
			name: name.to_string(),
			start: start.to_string(),
			end: end.to_string(),
		}
	}

	fn fresh(reached: &str) -> Fresh {
		Fresh {
			reached: reached.to_string(),
			covered: 0,
			play: Vec::new(),
		}
	}

	fn take(fresh: &mut Fresh, incremental: &Incremental) {
		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		taken(&db, incremental, OWN, fresh).unwrap_or_else(|e| unreachable!("{e}"));
	}

	#[test]
	fn a_dump_starting_where_the_count_stopped_adds_what_it_holds_of_ours() {
		let dir = dump("folded", &[listen(OWN, AAAA, "2026-08-21 10:00:00")]);
		let mut fresh = fresh("2026-08-21 00:00:03.155180+00:00");

		take(
			&mut fresh,
			&incremental(
				"listenbrainz-dump-2026-08-22",
				"2026-08-21 00:00:03.155180+00:00",
				"2026-08-22 00:00:02.641933+00:00",
				dir.clone(),
			),
		);

		assert_eq!(fresh.play.len(), 1);
		assert_eq!(fresh.reached, "2026-08-22 00:00:02.641933+00:00");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_dump_reaching_no_further_than_the_count_is_skipped_rather_than_counted_twice() {
		let dir = dump("twice", &[listen(OWN, AAAA, "2026-07-11 10:00:00")]);
		let mut fresh = fresh("2026-07-12 00:00:04.001868+00:00");

		take(
			&mut fresh,
			&incremental(
				"listenbrainz-dump-2026-07-12",
				"2026-07-11 00:00:02.000000+00:00",
				"2026-07-12 00:00:02.000000+00:00",
				dir.clone(),
			),
		);

		assert!(fresh.play.is_empty());
		assert_eq!(fresh.reached, "2026-07-12 00:00:04.001868+00:00");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_dump_reaching_back_into_the_count_and_past_it_leaves_its_whole_window_out() {
		let dir = dump("straddle", &[listen(OWN, AAAA, "2026-07-12 10:00:00")]);
		let mut fresh = fresh("2026-07-12 00:00:04.001868+00:00");

		take(
			&mut fresh,
			&incremental(
				"listenbrainz-dump-2026-07-13",
				"2026-07-12 00:00:02.000000+00:00",
				"2026-07-13 00:00:02.000000+00:00",
				dir.clone(),
			),
		);

		assert!(fresh.play.is_empty());
		assert_eq!(fresh.reached, "2026-07-13 00:00:02.000000+00:00");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_dump_starting_before_the_count_stopped_is_behind_it() {
		assert!(behind(
			"2026-07-12 00:00:04.001868+00:00",
			"2026-07-12 00:00:02.000000+00:00"
		));
		assert!(!behind(
			"2026-07-12 00:00:04.001868+00:00",
			"2026-07-12 00:00:04.001868+00:00"
		));
		assert!(!behind(
			"2026-07-12 00:00:04.001868+00:00",
			"2026-07-23 00:00:03.690928+00:00"
		));
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
}
