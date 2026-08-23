use std::path::Path;

use ansi::abbrev::{B, D, Y};

use super::{
	super::{
		dump::{self, Incremental},
		index::Gap,
		progress,
	},
	delta,
	work::{self, Reach},
};

pub(super) fn taken(
	db: &duckdb::Connection,
	work: &Path,
	reach: &mut Reach,
	incremental: &Incremental,
) -> hmerr::Result<()> {
	let covered = dump::reach(&reach.covered)?;
	let start = dump::reach(&incremental.start)?;

	if start < covered {
		return overlapping(work, reach, incremental, covered);
	}

	if start > covered {
		lost(reach, &incremental.start);
	}

	delta::fold(db, work, incremental)?;

	reach.covered.clone_from(&incremental.end);
	reach.absorbed += 1;

	work::reached(work, reach)
}

fn lost(reach: &mut Reach, start: &str) {
	progress::say(format!(
		"{Y}nothing published covers {B}{from}{D}{Y} to {B}{to}{D}{Y}, \
		those listens are out of reach{D}",
		from = reach.covered,
		to = start
	));

	reach.gap.push(Gap {
		from: reach.covered.clone(),
		to: start.to_string(),
	});
}

fn overlapping(
	work: &Path,
	reach: &mut Reach,
	incremental: &Incremental,
	covered: u64,
) -> hmerr::Result<()> {
	progress::say(format!(
		"{Y}{B}{name}{D}{Y} reaches back into what the index already holds, \
		skipped rather than counted twice{D}",
		name = incremental.name
	));

	if dump::reach(&incremental.end)? <= covered {
		return Ok(());
	}

	lost(reach, &incremental.end);
	reach.covered.clone_from(&incremental.end);

	work::reached(work, reach)
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::{
		super::{
			super::index,
			fixture::{
				BEFORE_THE_INDEX, BUILT, NEXT, POOLED, absorb, built, day, incremental, plays,
			},
		},
		*,
	};

	#[test]
	fn a_dump_reaching_back_into_the_index_is_never_counted_twice() {
		let (dir, index, meta) = built("overlap");
		let held = plays(&index, POOLED, 0);

		let reach = absorb(&index, &meta, &incremental(&dir, BEFORE_THE_INDEX, &day()))
			.unwrap_or_else(|_| unreachable!());

		assert_eq!(plays(&index, POOLED, 0), held);
		assert_eq!(reach.absorbed, 0);
		assert_eq!(reach.gap.len(), 1);
		assert_eq!(reach.covered, NEXT);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn absorbing_the_same_dump_again_never_counts_it_twice() {
		let (dir, index, meta) = built("idempotent");

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));
		let meta = index::meta::read(&index).unwrap_or_else(|_| unreachable!());
		let once = plays(&index, POOLED, 0);

		let work = work::open(&index, meta.covered()).unwrap_or_default();
		let db = index::session::of(&work).unwrap_or_else(|_| unreachable!());
		let mut reach = work::reach(&work, &meta);
		let again = incremental(&dir, BUILT, &day());

		assert!(taken(&db, &work, &mut reach, &again).is_ok());

		assert_eq!(
			reach.absorbed, 0,
			"a dump the index already reached is skipped"
		);
		assert_eq!(
			reach.covered,
			meta.covered(),
			"what the index reaches never moves backwards"
		);
		assert!(
			reach.gap.is_empty(),
			"a dump holding no window of its own leaves no hole to repair"
		);
		assert_eq!(plays(&index, POOLED, 0), once);
		let _ = fs::remove_dir_all(&dir);
	}
}
