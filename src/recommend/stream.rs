use super::{feed::Feed, recommendation::Recommendation, skip::Skip};

pub(super) struct Stream {
	feed: Vec<Option<Box<dyn Feed>>>,
	turn: usize,
	unlistened: bool,
	read: usize,
}

impl Stream {
	pub(super) fn new(feed: Vec<Box<dyn Feed>>, unlistened: bool) -> Self {
		Self {
			feed: feed.into_iter().map(Some).collect(),
			turn: 0,
			unlistened,
			read: 0,
		}
	}

	pub(super) fn next(
		&mut self,
		skip: &mut Skip,
	) -> hmerr::Result<Option<(usize, Recommendation)>> {
		while let Some(turn) = self.living() {
			let Some(recommendation) = self.pull(turn, skip)? else {
				self.retire(turn);
				continue;
			};

			let index = self.read;
			self.read += 1;
			self.turn = turn + 1;

			if self.unlistened && recommendation.origin.latest_listened_at().is_some() {
				continue;
			}

			if !skip.fresh(recommendation.mbid) {
				continue;
			}

			return Ok(Some((index, recommendation)));
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
		let count = self.feed.len();

		(0..count)
			.map(|step| (self.turn + step) % count.max(1))
			.find(|turn| self.feed.get(*turn).is_some_and(Option::is_some))
	}
}

#[cfg(test)]
mod tests {
	use std::collections::VecDeque;

	use chrono::{NaiveDate, Utc};

	use super::{super::recommendation::Origin, *};
	use crate::declaration::Source;

	struct Canned(VecDeque<Recommendation>);

	impl Feed for Canned {
		fn next(&mut self, _skip: &Skip) -> hmerr::Result<Option<Recommendation>> {
			Ok(self.0.pop_front())
		}
	}

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

	fn canned(recommendation: Vec<Recommendation>) -> Box<dyn Feed> {
		Box::new(Canned(recommendation.into()))
	}

	fn drain(stream: &mut Stream, skip: &mut Skip) -> Vec<u8> {
		let mut seen = Vec::new();

		while let Ok(Some((_, recommendation))) = stream.next(skip) {
			seen.push(recommendation.mbid.as_bytes()[0]);
		}

		seen
	}

	fn drain_index(stream: &mut Stream, skip: &mut Skip) -> Vec<usize> {
		let mut seen = Vec::new();

		while let Ok(Some((index, _))) = stream.next(skip) {
			seen.push(index);
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
}
