use crate::{ask, recommend::island::index::own};

use super::{
	super::{
		cache,
		fetch::{Listen, ListenCount},
	},
	Held,
};

pub(super) fn folded(username: &str, held: &mut Held) -> hmerr::Result<()> {
	let reached = held.reach().to_string();

	own::fresh(username, &reached, &ask::Terminal, &mut |fold| {
		absorbed(held, fold);

		cache::dump::write(username, held)
	})
}

pub(super) fn absorbed(held: &mut Held, fold: own::Fold) {
	merge(held.fold.get_or_insert_default(), fold.play);
	held.covered = held.covered.max(fold.covered);
	held.gap.extend(fold.gap);
	held.reached = fold.reached;
}

fn merge(count: &mut ListenCount, play: Vec<own::Play>) {
	for play in play {
		let listen = count.entry(play.mbid).or_insert_with(|| Listen {
			count: 0,
			track: play.track,
			artist: play.artist,
		});

		listen.count = listen.count.saturating_add(play.plays);
	}
}

#[cfg(test)]
mod tests {
	use crate::recommend::island::index::own::Gap;

	use super::{
		super::fixture::{LATEST, MBID, NEWER, fold, held, play, plays},
		*,
	};

	#[test]
	fn what_the_dump_counted_and_what_was_folded_onto_it_add_up() {
		let mut held = Held {
			count: ListenCount::from([(
				MBID.parse().unwrap_or_default(),
				Listen {
					count: 30,
					track: "Fairy Dance".to_string(),
					artist: "UNDEAD CORPORATION".to_string(),
				},
			)]),
			..held()
		};

		absorbed(&mut held, fold(LATEST, 5, Vec::new()));

		assert_eq!(plays(&held, MBID), Some(35));
	}

	#[test]
	fn every_incremental_lands_on_the_count_as_it_is_read_not_once_the_chain_is_over() {
		let mut held = held();

		absorbed(&mut held, fold(NEWER, 40, Vec::new()));

		assert_eq!(held.reach(), NEWER);
		assert_eq!(plays(&held, MBID), Some(40));

		absorbed(
			&mut held,
			fold(
				LATEST,
				2,
				vec![Gap {
					from: NEWER.to_string(),
					to: LATEST.to_string(),
				}],
			),
		);

		assert_eq!(held.reach(), LATEST);
		assert_eq!(plays(&held, MBID), Some(42));
		assert_eq!(held.gap.len(), 1);
	}

	#[test]
	fn what_an_incremental_adds_lands_on_the_count_the_dump_left() {
		const FRESH: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

		let mut count = ListenCount::new();
		merge(&mut count, vec![play(MBID, 40)]);
		merge(&mut count, vec![play(MBID, 2), play(FRESH, 7)]);

		assert_eq!(
			count
				.get(&MBID.parse().unwrap_or_default())
				.map(|l| l.count),
			Some(42)
		);
		assert_eq!(
			count
				.get(&FRESH.parse().unwrap_or_default())
				.map(|l| l.count),
			Some(7)
		);
		assert_eq!(
			count
				.get(&FRESH.parse().unwrap_or_default())
				.map(|l| l.track.clone()),
			Some("Fairy Dance".to_string())
		);
	}
}
