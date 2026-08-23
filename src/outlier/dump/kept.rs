use super::{
	Held,
	held::Carried,
	say::{another_dump, merged_in, stuck_at},
};

pub(super) enum Kept {
	Cached(Held),
	Rescan(Rescan),
}

pub(super) enum Rescan {
	Fresh,
	Another(String),
	Merged,
	Stuck(String),
	Carried(Carried),
}

pub(super) fn kept(held: Option<Held>, unpacked: Option<&str>, refresh: bool) -> Kept {
	let Some(held) = held else {
		return Kept::Rescan(Rescan::Fresh);
	};

	let Some(unpacked) = unpacked else {
		return Kept::Cached(held);
	};

	if held.dump != unpacked {
		return Kept::Rescan(Rescan::Another(unpacked.to_string()));
	}

	if !refresh {
		return Kept::Cached(held);
	}

	if !held.apart() {
		return Kept::Rescan(Rescan::Merged);
	}

	if !held.foldable() {
		return Kept::Rescan(Rescan::Stuck(held.reach().to_string()));
	}

	Kept::Rescan(Rescan::Carried(held.carried()))
}

pub(super) fn told(rescan: Rescan) -> Option<Carried> {
	match rescan {
		Rescan::Carried(carried) => Some(carried),
		Rescan::Fresh => None,
		Rescan::Another(dump) => {
			another_dump(&dump);
			None
		}
		Rescan::Merged => {
			merged_in();
			None
		}
		Rescan::Stuck(reached) => {
			stuck_at(&reached);
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{
		super::{
			super::fetch::{Listen, ListenCount},
			fixture::{DUMP, LATEST, MBID, NEWER, fold, held},
			fold::absorbed,
		},
		*,
	};

	fn cached_dump(kept: Kept) -> Option<String> {
		match kept {
			Kept::Cached(held) => Some(held.dump),
			Kept::Rescan(_) => None,
		}
	}

	fn carried(kept: Kept) -> Option<Carried> {
		match kept {
			Kept::Rescan(Rescan::Carried(carried)) => Some(carried),
			_ => None,
		}
	}

	#[test]
	fn what_was_read_off_the_dump_that_is_still_unpacked_is_read_again_from_the_cache() {
		assert_eq!(
			cached_dump(kept(Some(held()), Some(DUMP), false)),
			Some(DUMP.into())
		);
	}

	#[test]
	fn a_newer_unpacked_dump_is_read_rather_than_what_the_cache_holds() {
		assert!(cached_dump(kept(Some(held()), Some(NEWER), false)).is_none());
	}

	#[test]
	fn a_refresh_reads_the_unpacked_dump_again() {
		assert!(cached_dump(kept(Some(held()), Some(DUMP), true)).is_none());
	}

	#[test]
	fn a_refresh_reads_the_dump_again_without_dropping_what_was_folded_onto_it() {
		let mut folded = held();
		absorbed(&mut folded, fold(LATEST, 7, Vec::new()));

		let carried = carried(kept(Some(folded), Some(DUMP), true))
			.unwrap_or_else(|| unreachable!("a refresh of the same dump carries the fold over"));

		assert_eq!(carried.reached, LATEST);
		assert_eq!(
			carried
				.fold
				.get(&MBID.parse().unwrap_or_default())
				.map(|l| l.count),
			Some(7)
		);
	}

	#[test]
	fn an_incremental_that_added_no_play_still_leaves_the_dump_and_the_fold_apart() {
		let mut quiet = held();
		absorbed(
			&mut quiet,
			crate::recommend::island::index::own::Fold {
				reached: LATEST.to_string(),
				covered: 0,
				play: Vec::new(),
				gap: Vec::new(),
			},
		);

		assert_eq!(quiet.reach(), LATEST);

		let carried = carried(kept(Some(quiet), Some(DUMP), true)).unwrap_or_else(|| {
			unreachable!("an incremental holding nothing of ours is still an incremental read")
		});

		assert_eq!(carried.reached, LATEST);
	}

	#[test]
	fn a_refresh_clears_counts_stopped_at_a_stamp_no_dump_can_be_held_against() {
		let mut wedged = held();
		absorbed(&mut wedged, fold(LATEST, 7, Vec::new()));
		wedged.reached = "END_TIMESTAMP".to_string();

		assert!(!wedged.foldable());
		assert!(carried(kept(Some(wedged), Some(DUMP), true)).is_none());
	}

	#[test]
	fn a_cache_written_before_the_fold_was_kept_apart_is_read_from_the_dump_up_again() {
		let merged = Held {
			reached: LATEST.to_string(),
			fold: None,
			count: ListenCount::from([(
				MBID.parse().unwrap_or_default(),
				Listen {
					count: 47,
					track: String::new(),
					artist: String::new(),
				},
			)]),
			..held()
		};

		assert!(carried(kept(Some(merged), Some(DUMP), true)).is_none());
	}

	#[test]
	fn an_unpacked_dump_the_counts_never_came_from_is_read_as_that_and_nothing_else() {
		let merged = Held {
			reached: LATEST.to_string(),
			fold: None,
			..held()
		};

		assert!(matches!(
			kept(Some(merged), Some(NEWER), false),
			Kept::Rescan(Rescan::Another(unpacked)) if unpacked == NEWER
		));
	}

	#[test]
	fn a_refresh_of_a_cache_that_cannot_tell_its_fold_apart_is_read_as_that() {
		let merged = Held {
			reached: LATEST.to_string(),
			fold: None,
			..held()
		};

		assert!(matches!(
			kept(Some(merged), Some(DUMP), true),
			Kept::Rescan(Rescan::Merged)
		));
	}

	#[test]
	fn a_refresh_of_counts_stopped_at_an_unreadable_stamp_says_where_they_stopped() {
		let mut wedged = held();
		absorbed(&mut wedged, fold(LATEST, 7, Vec::new()));
		wedged.reached = "END_TIMESTAMP".to_string();

		assert!(matches!(
			kept(Some(wedged), Some(DUMP), true),
			Kept::Rescan(Rescan::Stuck(reached)) if reached == "END_TIMESTAMP"
		));
	}

	#[test]
	fn a_newer_dump_is_a_baseline_of_its_own_with_nothing_folded_onto_it_yet() {
		let mut folded = held();
		absorbed(&mut folded, fold(LATEST, 7, Vec::new()));

		assert!(carried(kept(Some(folded), Some(NEWER), false)).is_none());
	}

	#[test]
	fn a_discarded_dump_leaves_the_cache_as_the_only_thing_it_was_read_into() {
		assert_eq!(
			cached_dump(kept(Some(held()), None, false)),
			Some(DUMP.into())
		);
		assert_eq!(
			cached_dump(kept(Some(held()), None, true)),
			Some(DUMP.into())
		);
	}

	#[test]
	fn nothing_cached_is_nothing_to_keep() {
		assert!(cached_dump(kept(None, Some(DUMP), false)).is_none());
		assert!(cached_dump(kept(None, None, false)).is_none());
		assert!(carried(kept(None, Some(DUMP), true)).is_none());
	}
}
