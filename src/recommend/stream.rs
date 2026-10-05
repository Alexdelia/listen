use serde::Serialize;

use crate::declaration::Source;

use super::{
	feed::Feed,
	recommendation::Recommendation,
	skip::{Reason, Skip},
	turn,
};

pub(super) struct Shown {
	pub index: usize,
	pub turn: usize,
	pub recommendation: Recommendation,
}

#[derive(Serialize)]
pub(super) struct Skipped {
	pub index: usize,
	pub turn: usize,
	pub mbid: Source,
	pub reason: Reason,
}

pub(super) struct Stream {
	feed: Vec<Option<Box<dyn Feed>>>,
	turn: usize,
	unlistened: bool,
	read: usize,
	skipped: Vec<Skipped>,
}

impl Stream {
	pub(super) fn new(feed: Vec<Box<dyn Feed>>, unlistened: bool) -> Self {
		Self {
			feed: feed.into_iter().map(Some).collect(),
			turn: 0,
			unlistened,
			read: 0,
			skipped: Vec::new(),
		}
	}

	pub(super) fn skipped(&self) -> &[Skipped] {
		&self.skipped
	}

	pub(super) fn next(&mut self, skip: &mut Skip) -> hmerr::Result<Option<Shown>> {
		while let Some(turn) = self.living() {
			let Some(recommendation) = self.pull(turn, skip)? else {
				self.retire(turn);
				continue;
			};

			let index = self.read;
			self.read += 1;
			self.turn = turn + 1;
			let mbid = recommendation.mbid;

			let listened = self.unlistened && recommendation.origin.latest_listened_at().is_some();
			if let Some(reason) = listened
				.then_some(Reason::Listened)
				.or_else(|| skip.stale(mbid))
			{
				self.skipped.push(Skipped {
					index,
					turn,
					mbid,
					reason,
				});
				continue;
			}

			skip.fresh(mbid);

			return Ok(Some(Shown {
				index,
				turn,
				recommendation,
			}));
		}

		Ok(None)
	}

	fn pull(&mut self, turn: usize, skip: &Skip) -> hmerr::Result<Option<Recommendation>> {
		match self.feed.get_mut(turn) {
			Some(Some(feed)) => feed.next(skip),
			_ => Ok(None),
		}
	}

	fn retire(&mut self, turn: usize) {
		if let Some(feed) = self.feed.get_mut(turn) {
			*feed = None;
		}
	}

	fn living(&self) -> Option<usize> {
		turn::living(self.turn, self.feed.len(), |turn| {
			self.feed.get(turn).is_some_and(Option::is_some)
		})
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashSet;

	use chrono::{NaiveDate, Utc};

	use super::{
		super::{feed::canned, recommendation::Origin},
		*,
	};
	use crate::declaration::Source;

	fn week() -> NaiveDate {
		NaiveDate::from_ymd_opt(2026, 7, 12).unwrap_or_default()
	}

	fn mbid(nibble: u8) -> Source {
		Source::from_bytes([nibble; 16])
	}

	fn weekly(nibble: u8) -> Recommendation {
		Recommendation {
			mbid: mbid(nibble),
			origin: Origin::WeeklyExploration {
				week: week(),
				position: nibble.into(),
			},
		}
	}

	fn cf(nibble: u8) -> Recommendation {
		Recommendation {
			mbid: mbid(nibble),
			origin: Origin::CollaborativeFiltering {
				position: nibble.into(),
				score: 1.0,
				latest_listened_at: None,
			},
		}
	}

	fn listened_cf(nibble: u8) -> Recommendation {
		Recommendation {
			mbid: mbid(nibble),
			origin: Origin::CollaborativeFiltering {
				position: nibble.into(),
				score: 1.0,
				latest_listened_at: Some(Utc::now()),
			},
		}
	}

	fn drain(stream: &mut Stream, skip: &mut Skip) -> Vec<u8> {
		let mut seen = Vec::new();

		while let Ok(Some(shown)) = stream.next(skip) {
			seen.push(shown.recommendation.mbid.as_bytes()[0]);
		}

		seen
	}

	fn drain_index(stream: &mut Stream, skip: &mut Skip) -> Vec<usize> {
		let mut seen = Vec::new();

		while let Ok(Some(shown)) = stream.next(skip) {
			seen.push(shown.index);
		}

		seen
	}

	#[test]
	fn the_first_feed_goes_first_then_all_alternate() {
		let mut stream = Stream::new(
			vec![
				canned(vec![weekly(1), weekly(2), weekly(3)]),
				canned(vec![cf(4), cf(5), cf(6)]),
			],
			false,
		);

		assert_eq!(
			drain(&mut stream, &mut Skip::default()),
			vec![1, 4, 2, 5, 3, 6]
		);
	}

	#[test]
	fn three_feeds_take_turns_in_order() {
		let mut stream = Stream::new(
			vec![
				canned(vec![weekly(1), weekly(2)]),
				canned(vec![cf(3), cf(4)]),
				canned(vec![cf(5), cf(6)]),
			],
			false,
		);

		assert_eq!(
			drain(&mut stream, &mut Skip::default()),
			vec![1, 3, 5, 2, 4, 6]
		);
	}

	#[test]
	fn a_drained_feed_leaves_the_others_alone() {
		let mut stream = Stream::new(
			vec![canned(vec![weekly(1)]), canned(vec![cf(4), cf(5)])],
			false,
		);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![1, 4, 5]);
	}

	#[test]
	fn a_drained_last_feed_leaves_the_first_alone() {
		let mut stream = Stream::new(
			vec![canned(vec![weekly(1), weekly(2)]), canned(vec![cf(4)])],
			false,
		);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![1, 4, 2]);
	}

	#[test]
	fn a_skipped_recommendation_still_spends_its_turn() {
		let mut skip = Skip::default();
		skip.fresh(mbid(4));

		let mut stream = Stream::new(
			vec![
				canned(vec![weekly(1), weekly(2)]),
				canned(vec![cf(4), cf(5)]),
			],
			false,
		);

		assert_eq!(drain(&mut stream, &mut skip), vec![1, 2, 5]);
	}

	#[test]
	fn the_index_counts_every_entry_the_stream_reads() {
		let mut skip = Skip::default();
		skip.fresh(mbid(4));

		let mut stream = Stream::new(
			vec![
				canned(vec![weekly(1), weekly(2)]),
				canned(vec![cf(4), cf(5)]),
			],
			false,
		);

		assert_eq!(drain_index(&mut stream, &mut skip), vec![0, 2, 3]);
	}

	#[test]
	fn a_recommendation_is_never_shown_twice() {
		let mut stream = Stream::new(
			vec![canned(vec![weekly(1)]), canned(vec![cf(1), cf(5)])],
			false,
		);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![1, 5]);
	}

	#[test]
	fn unlistened_drops_listened_collaborative_filtering_and_keeps_weekly() {
		let mut stream = Stream::new(
			vec![
				canned(vec![weekly(1), weekly(2)]),
				canned(vec![listened_cf(4), cf(5)]),
			],
			true,
		);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![1, 2, 5]);
	}

	#[test]
	fn a_single_feed_needs_no_alternation() {
		let mut stream = Stream::new(vec![canned(vec![cf(4), cf(5)])], false);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![4, 5]);
	}

	#[test]
	fn no_feed_yields_nothing() {
		let mut stream = Stream::new(Vec::new(), false);

		assert!(drain(&mut stream, &mut Skip::default()).is_empty());
	}

	#[test]
	fn every_shown_entry_carries_the_turn_of_its_feed() {
		let mut stream = Stream::new(
			vec![canned(vec![weekly(1), weekly(2)]), canned(vec![cf(4)])],
			false,
		);
		let mut turn = Vec::new();

		while let Ok(Some(shown)) = stream.next(&mut Skip::default()) {
			turn.push(shown.turn);
		}

		assert_eq!(turn, vec![0, 1, 0]);
	}

	#[test]
	fn a_duplicate_is_traced_as_skipped() {
		let mut stream = Stream::new(
			vec![canned(vec![weekly(1)]), canned(vec![cf(1), cf(5)])],
			false,
		);
		drain(&mut stream, &mut Skip::default());

		let skipped: Vec<(usize, usize, Reason)> = stream
			.skipped()
			.iter()
			.map(|skipped| (skipped.index, skipped.turn, skipped.reason))
			.collect();

		assert_eq!(skipped, vec![(1, 1, Reason::Duplicate)]);
	}

	#[test]
	fn a_declared_entry_is_traced_as_declared() {
		let mut skip = Skip::new(HashSet::from([mbid(4)]), HashSet::new());
		let mut stream = Stream::new(vec![canned(vec![cf(4), cf(5)])], false);
		drain(&mut stream, &mut skip);

		assert_eq!(
			stream
				.skipped()
				.first()
				.map(|skipped| (skipped.mbid, skipped.reason)),
			Some((mbid(4), Reason::Declared))
		);
	}

	#[test]
	fn a_listened_entry_is_traced_as_listened_when_unlistened() {
		let mut stream = Stream::new(vec![canned(vec![listened_cf(4), cf(5)])], true);
		drain(&mut stream, &mut Skip::default());

		assert_eq!(
			stream.skipped().first().map(|skipped| skipped.reason),
			Some(Reason::Listened)
		);
	}
}
