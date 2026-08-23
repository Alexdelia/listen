use std::path::Path;

use ansi::abbrev::{B, D, Y};

use super::{
	super::{
		board::Board,
		dump::{self, Incremental, Pending},
		progress,
	},
	Fold, Gap, reach, scan,
	stage::{self, Stage},
	stamped,
};

pub(super) fn run(
	db: &duckdb::Connection,
	root: &Path,
	pending: &[&Pending],
	own: u32,
	reached: &str,
	keep: &mut impl FnMut(Fold) -> hmerr::Result<()>,
) -> hmerr::Result<()> {
	let planned = Board::of(&stage::PLAN, &stage::chain(pending))?;

	let downloading = planned.start(Stage::Download)?;
	let verifying = planned.start(Stage::Verify)?;
	let unpacking = planned.start(Stage::Unpack)?;
	let reading = planned.start(Stage::Listen)?;

	let mut reached = reached.to_string();

	for pending in pending {
		dump::pull(root, pending, &downloading, &verifying)?;
		let incremental = dump::opened(root, pending, &unpacking)?;

		let taken = taken(db, &incremental, own, &reached)?;
		reading.inc(1);
		dump::release(&incremental)?;

		if let Some(taken) = taken {
			reached.clone_from(&taken.reached);
			keep(taken)?;
		}
	}

	Ok(())
}

fn taken(
	db: &duckdb::Connection,
	incremental: &Incremental,
	own: u32,
	reached: &str,
) -> hmerr::Result<Option<Fold>> {
	if !stamped(&incremental.start) || !stamped(&incremental.end) {
		unstamped(&incremental.name);
		return Ok(None);
	}

	if reach::behind(reached, &incremental.start) {
		return Ok(skipped(incremental, reached));
	}

	let mut gap = Vec::new();

	if reach::lost(reached, &incremental.start) {
		out_of_reach(reached, &incremental.start);
		gap.push(missed(reached, &incremental.start));
	}

	let scanned = scan::of(db, &incremental.dir, own)?;

	Ok(Some(Fold {
		reached: incremental.end.clone(),
		covered: scanned.covered,
		play: scanned.play,
		gap,
	}))
}

fn skipped(incremental: &Incremental, reached: &str) -> Option<Fold> {
	already_counted(&incremental.name);

	if !reach::lost(reached, &incremental.end) {
		return None;
	}

	stays_out(reached, &incremental.end);

	Some(Fold {
		reached: incremental.end.clone(),
		covered: 0,
		play: Vec::new(),
		gap: vec![missed(reached, &incremental.end)],
	})
}

fn missed(reached: &str, to: &str) -> Gap {
	Gap {
		from: reached.to_string(),
		to: to.to_string(),
	}
}

fn unstamped(name: &str) {
	progress::say(format!(
		"{Y}{B}{name}{D}{Y} carries no timestamp the counts can be held against, \
		nothing of it is folded in and the window it covers is left for the next one to miss{D}"
	));
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

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use super::{
		super::fixture::{AAAA, OWN, dump, listen},
		*,
	};

	fn incremental(name: &str, start: &str, end: &str, dir: PathBuf) -> Incremental {
		Incremental {
			dir,
			name: name.to_string(),
			start: start.to_string(),
			end: end.to_string(),
		}
	}

	fn take(reached: &str, incremental: &Incremental) -> Option<Fold> {
		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		taken(&db, incremental, OWN, reached).unwrap_or_else(|e| unreachable!("{e}"))
	}

	fn window(fold: &Fold) -> Vec<(String, String)> {
		fold.gap
			.iter()
			.map(|gap| (gap.from.clone(), gap.to.clone()))
			.collect()
	}

	#[test]
	fn a_dump_starting_where_the_count_stopped_adds_what_it_holds_of_ours() {
		let dir = dump("folded", &[listen(OWN, AAAA, "2026-08-21 10:00:00")]);

		let taken = take(
			"2026-08-21 00:00:03.155180+00:00",
			&incremental(
				"listenbrainz-dump-2026-08-22",
				"2026-08-21 00:00:03.155180+00:00",
				"2026-08-22 00:00:02.641933+00:00",
				dir.clone(),
			),
		)
		.unwrap_or_else(|| unreachable!());

		assert_eq!(taken.play.len(), 1);
		assert_eq!(taken.reached, "2026-08-22 00:00:02.641933+00:00");
		assert!(window(&taken).is_empty());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_dump_reaching_no_further_than_the_count_is_skipped_rather_than_counted_twice() {
		let dir = dump("twice", &[listen(OWN, AAAA, "2026-07-11 10:00:00")]);

		let taken = take(
			"2026-07-12 00:00:04.001868+00:00",
			&incremental(
				"listenbrainz-dump-2026-07-12",
				"2026-07-11 00:00:02.000000+00:00",
				"2026-07-12 00:00:02.000000+00:00",
				dir.clone(),
			),
		);

		assert!(taken.is_none());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_dump_reaching_back_into_the_count_and_past_it_leaves_its_whole_window_out() {
		let dir = dump("straddle", &[listen(OWN, AAAA, "2026-07-12 10:00:00")]);

		let taken = take(
			"2026-07-12 00:00:04.001868+00:00",
			&incremental(
				"listenbrainz-dump-2026-07-13",
				"2026-07-12 00:00:02.000000+00:00",
				"2026-07-13 00:00:02.000000+00:00",
				dir.clone(),
			),
		)
		.unwrap_or_else(|| unreachable!());

		assert!(taken.play.is_empty());
		assert_eq!(taken.reached, "2026-07-13 00:00:02.000000+00:00");
		assert_eq!(
			window(&taken),
			[(
				"2026-07-12 00:00:04.001868+00:00".to_string(),
				"2026-07-13 00:00:02.000000+00:00".to_string()
			)]
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_dump_whose_window_carries_no_timestamp_is_left_out_rather_than_stopping_the_count_dead() {
		let dir = dump("unstamped", &[listen(OWN, AAAA, "2026-08-21 10:00:00")]);

		let taken = take(
			"2026-08-21 00:00:03.155180+00:00",
			&incremental(
				"listenbrainz-dump-2026-08-22",
				"2026-08-21 00:00:03.155180+00:00",
				"END_TIMESTAMP",
				dir.clone(),
			),
		);

		assert!(taken.is_none());

		let taken = take(
			"2026-08-21 00:00:03.155180+00:00",
			&incremental(
				"listenbrainz-dump-2026-08-22",
				"START_TIMESTAMP",
				"2026-08-22 00:00:02.641933+00:00",
				dir.clone(),
			),
		);

		assert!(taken.is_none());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_window_no_dump_covers_is_written_down_as_a_hole_in_the_count() {
		let dir = dump("holed", &[listen(OWN, AAAA, "2026-07-23 10:00:00")]);

		let taken = take(
			"2026-07-12 00:00:04.001868+00:00",
			&incremental(
				"listenbrainz-dump-2026-07-24",
				"2026-07-23 00:00:03.690928+00:00",
				"2026-07-24 00:00:02.000000+00:00",
				dir.clone(),
			),
		)
		.unwrap_or_else(|| unreachable!());

		assert_eq!(taken.play.len(), 1);
		assert_eq!(taken.reached, "2026-07-24 00:00:02.000000+00:00");
		assert_eq!(
			window(&taken),
			[(
				"2026-07-12 00:00:04.001868+00:00".to_string(),
				"2026-07-23 00:00:03.690928+00:00".to_string()
			)]
		);
		let _ = fs::remove_dir_all(&dir);
	}
}
