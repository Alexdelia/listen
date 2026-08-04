use super::{feed::Feed, recommendation::Recommendation, skip::Skip};

#[derive(Clone, Copy)]
enum Turn {
	Weekly,
	CollaborativeFiltering,
}

impl Turn {
	fn other(self) -> Self {
		match self {
			Self::Weekly => Self::CollaborativeFiltering,
			Self::CollaborativeFiltering => Self::Weekly,
		}
	}
}

pub(super) struct Stream {
	weekly: Option<Box<dyn Feed>>,
	collaborative_filtering: Option<Box<dyn Feed>>,
	turn: Turn,
	unlistened: bool,
}

impl Stream {
	pub(super) fn new(
		weekly: Option<Box<dyn Feed>>,
		collaborative_filtering: Option<Box<dyn Feed>>,
		unlistened: bool,
	) -> Self {
		Self {
			weekly,
			collaborative_filtering,
			turn: Turn::Weekly,
			unlistened,
		}
	}

	pub(super) fn next(&mut self, skip: &mut Skip) -> hmerr::Result<Option<Recommendation>> {
		while let Some(turn) = self.living() {
			let Some(recommendation) = self.pull(turn)? else {
				*self.feed(turn) = None;
				continue;
			};

			if self.unlistened && recommendation.origin.latest_listened_at().is_some() {
				continue;
			}

			if !skip.fresh(recommendation.mbid) {
				continue;
			}

			self.turn = turn.other();

			return Ok(Some(recommendation));
		}

		Ok(None)
	}

	fn pull(&mut self, turn: Turn) -> hmerr::Result<Option<Recommendation>> {
		match self.feed(turn) {
			Some(feed) => feed.next(),
			None => Ok(None),
		}
	}

	fn living(&self) -> Option<Turn> {
		[self.turn, self.turn.other()]
			.into_iter()
			.find(|turn| match turn {
				Turn::Weekly => self.weekly.is_some(),
				Turn::CollaborativeFiltering => self.collaborative_filtering.is_some(),
			})
	}

	fn feed(&mut self, turn: Turn) -> &mut Option<Box<dyn Feed>> {
		match turn {
			Turn::Weekly => &mut self.weekly,
			Turn::CollaborativeFiltering => &mut self.collaborative_filtering,
		}
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
		fn next(&mut self) -> hmerr::Result<Option<Recommendation>> {
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
				score: 1.0,
				latest_listened_at: None,
			},
		}
	}

	fn listened_cf(nibble: u8) -> Recommendation {
		Recommendation {
			mbid: mbid(nibble),
			origin: Origin::CollaborativeFiltering {
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

		while let Ok(Some(recommendation)) = stream.next(skip) {
			seen.push(recommendation.mbid.as_bytes()[0]);
		}

		seen
	}

	#[test]
	fn weekly_goes_first_then_both_alternate() {
		let mut stream = Stream::new(
			Some(canned(vec![weekly(1), weekly(2), weekly(3)])),
			Some(canned(vec![cf(4), cf(5), cf(6)])),
			false,
		);

		assert_eq!(
			drain(&mut stream, &mut Skip::default()),
			vec![1, 4, 2, 5, 3, 6]
		);
	}

	#[test]
	fn a_drained_weekly_leaves_collaborative_filtering_alone() {
		let mut stream = Stream::new(
			Some(canned(vec![weekly(1)])),
			Some(canned(vec![cf(4), cf(5)])),
			false,
		);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![1, 4, 5]);
	}

	#[test]
	fn a_drained_collaborative_filtering_leaves_weekly_alone() {
		let mut stream = Stream::new(
			Some(canned(vec![weekly(1), weekly(2)])),
			Some(canned(vec![cf(4)])),
			false,
		);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![1, 4, 2]);
	}

	#[test]
	fn a_skipped_recommendation_does_not_spend_a_turn() {
		let mut skip = Skip::default();
		skip.fresh(mbid(4));

		let mut stream = Stream::new(
			Some(canned(vec![weekly(1), weekly(2)])),
			Some(canned(vec![cf(4), cf(5)])),
			false,
		);

		assert_eq!(drain(&mut stream, &mut skip), vec![1, 5, 2]);
	}

	#[test]
	fn a_recommendation_is_never_shown_twice() {
		let mut stream = Stream::new(
			Some(canned(vec![weekly(1)])),
			Some(canned(vec![cf(1), cf(5)])),
			false,
		);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![1, 5]);
	}

	#[test]
	fn unlistened_drops_listened_collaborative_filtering_and_keeps_weekly() {
		let mut stream = Stream::new(
			Some(canned(vec![weekly(1), weekly(2)])),
			Some(canned(vec![listened_cf(4), cf(5)])),
			true,
		);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![1, 5, 2]);
	}

	#[test]
	fn a_single_source_needs_no_alternation() {
		let mut stream = Stream::new(None, Some(canned(vec![cf(4), cf(5)])), false);

		assert_eq!(drain(&mut stream, &mut Skip::default()), vec![4, 5]);
	}
}
