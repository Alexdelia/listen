use std::path::Path;

use ansi::abbrev::{D, F, Y};

use super::{
	super::{
		index::{self, Meta},
		progress,
	},
	work::Reach,
};

pub(super) fn skipped(dir: &Path, held: &Meta, reach: &Reach) -> hmerr::Result<()> {
	progress::say(format!("{F}nothing absorbed{D}"));

	if reach.through == held.through && reach.gap.len() == held.gap.len() {
		return Ok(());
	}

	index::meta::write(
		dir,
		&Meta {
			reached: Some(reach.covered.clone()),
			through: reach.through,
			gap: reach.gap.clone(),
			absorbed: held.absorbed + reach.absorbed,
			..held.clone()
		},
	)?;

	if reach.gap.len() == held.gap.len() {
		return Ok(());
	}

	progress::say(format!(
		"{Y}the window those dumps left uncovered is recorded on the index{D}"
	));

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::{
		super::{
			fixture::{BEFORE_THE_INDEX, NEXT, built, day, incremental, listenless},
			reach::taken,
			work::{self, LIBRARY},
		},
		*,
	};

	#[test]
	fn a_chain_that_folded_nothing_still_records_the_window_it_skipped() {
		let (dir, index, meta) = built("skipped");
		let work = work::open(&index, meta.covered()).unwrap_or_default();
		let db = index::session::of(&work).unwrap_or_else(|_| unreachable!());
		let mut reach = work::reach(&work, &meta);

		taken(
			&db,
			&work,
			&mut reach,
			&incremental(&dir, BEFORE_THE_INDEX, &day()),
		)
		.unwrap_or_else(|e| unreachable!("{e}"));

		assert!(!work::folded(&work, LIBRARY));
		skipped(&index, &meta, &reach).unwrap_or_else(|e| unreachable!("{e}"));

		let now = index::meta::read(&index).unwrap_or_else(|_| unreachable!());

		assert_eq!(now.covered(), NEXT);
		assert_eq!(now.gap.len(), 1);
		assert_eq!(now.absorbed, 0);
		assert_eq!(
			now.recording, meta.recording,
			"the parts it published nothing over never moved"
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_chain_of_listenless_dumps_still_carries_the_index_through_them() {
		let (dir, index, meta) = built("listenless_chain");
		let work = work::open(&index, meta.covered()).unwrap_or_default();
		let db = index::session::of(&work).unwrap_or_else(|_| unreachable!());
		let mut reach = work::reach(&work, &meta);

		taken(&db, &work, &mut reach, &listenless(&dir)).unwrap_or_else(|e| unreachable!("{e}"));

		assert!(!work::folded(&work, LIBRARY));
		skipped(&index, &meta, &reach).unwrap_or_else(|e| unreachable!("{e}"));

		let now = index::meta::read(&index).unwrap_or_else(|_| unreachable!());

		assert_eq!(now.covered(), NEXT);
		assert_eq!(now.through, Some(2594));
		assert!(now.gap.is_empty());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_chain_that_skipped_nothing_leaves_the_index_as_it_stands() {
		let (dir, index, meta) = built("quiet");
		let work = work::open(&index, meta.covered()).unwrap_or_default();
		let reach = work::reach(&work, &meta);

		skipped(&index, &meta, &reach).unwrap_or_else(|e| unreachable!("{e}"));

		assert!(
			index::meta::read(&index)
				.unwrap_or_else(|_| unreachable!())
				.reached
				.is_none()
		);
		let _ = fs::remove_dir_all(&dir);
	}
}
