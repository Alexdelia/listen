use std::path::Path;

use ansi::abbrev::{B, D, Y};

use super::{
	super::{
		dump::{self, Incremental, Pending},
		progress,
	},
	Fresh, Gap, board, reach, scan,
};

pub(super) fn run(
	db: &duckdb::Connection,
	root: &Path,
	pending: &[&Pending],
	own: u32,
	reached: &str,
) -> hmerr::Result<Fresh> {
	let planned = board::of(&board::chain(pending))?;

	let downloading = board::start(&planned, board::Stage::Download)?;
	let verifying = board::start(&planned, board::Stage::Verify)?;
	let unpacking = board::start(&planned, board::Stage::Unpack)?;
	let reading = board::start(&planned, board::Stage::Listen)?;

	let mut fresh = Fresh {
		reached: reached.to_string(),
		covered: 0,
		play: Vec::new(),
		gap: Vec::new(),
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
	if reach::behind(&fresh.reached, &incremental.start) {
		skipped(incremental, fresh);
		return Ok(());
	}

	if reach::lost(&fresh.reached, &incremental.start) {
		out_of_reach(&fresh.reached, &incremental.start);
		missed(fresh, &incremental.start);
	}

	let scanned = scan::of(db, &incremental.dir, own)?;
	fresh.play.extend(scanned.play);
	fresh.covered = fresh.covered.max(scanned.covered);
	fresh.reached.clone_from(&incremental.end);

	Ok(())
}

fn skipped(incremental: &Incremental, fresh: &mut Fresh) {
	already_counted(&incremental.name);

	if !reach::lost(&fresh.reached, &incremental.end) {
		return;
	}

	stays_out(&fresh.reached, &incremental.end);
	missed(fresh, &incremental.end);
	fresh.reached.clone_from(&incremental.end);
}

fn missed(fresh: &mut Fresh, to: &str) {
	fresh.gap.push(Gap {
		from: fresh.reached.clone(),
		to: to.to_string(),
	});
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

	fn fresh(reached: &str) -> Fresh {
		Fresh {
			reached: reached.to_string(),
			covered: 0,
			play: Vec::new(),
			gap: Vec::new(),
		}
	}

	fn window(fresh: &Fresh) -> Vec<(String, String)> {
		fresh
			.gap
			.iter()
			.map(|gap| (gap.from.clone(), gap.to.clone()))
			.collect()
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
		assert!(window(&fresh).is_empty());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_window_no_dump_covers_is_written_down_as_a_hole_in_the_count() {
		let dir = dump("holed", &[listen(OWN, AAAA, "2026-07-23 10:00:00")]);
		let mut fresh = fresh("2026-07-12 00:00:04.001868+00:00");

		take(
			&mut fresh,
			&incremental(
				"listenbrainz-dump-2026-07-24",
				"2026-07-23 00:00:03.690928+00:00",
				"2026-07-24 00:00:02.000000+00:00",
				dir.clone(),
			),
		);

		assert_eq!(fresh.play.len(), 1);
		assert_eq!(fresh.reached, "2026-07-24 00:00:02.000000+00:00");
		assert_eq!(
			window(&fresh),
			[(
				"2026-07-12 00:00:04.001868+00:00".to_string(),
				"2026-07-23 00:00:03.690928+00:00".to_string()
			)]
		);
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
		assert!(window(&fresh).is_empty());
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
		assert_eq!(
			window(&fresh),
			[(
				"2026-07-12 00:00:04.001868+00:00".to_string(),
				"2026-07-13 00:00:02.000000+00:00".to_string()
			)]
		);
		let _ = fs::remove_dir_all(&dir);
	}
}
