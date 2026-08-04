use std::collections::VecDeque;

use super::{feed::Feed, recommendation::Recommendation};

pub(super) struct Queue(VecDeque<Recommendation>);

impl Queue {
	pub(super) fn new(recommendation: Vec<Recommendation>) -> Self {
		Self(recommendation.into())
	}
}

impl Feed for Queue {
	fn next(&mut self) -> hmerr::Result<Option<Recommendation>> {
		Ok(self.0.pop_front())
	}
}

#[cfg(test)]
mod tests {
	use chrono::NaiveDate;

	use super::super::recommendation::Origin;
	use super::*;
	use crate::declaration::Source;

	fn recommendation(nibble: u8) -> Recommendation {
		Recommendation {
			mbid: Source::from_bytes([nibble; 16]),
			origin: Origin::WeeklyExploration {
				week: NaiveDate::default(),
				position: nibble.into(),
			},
		}
	}

	#[test]
	fn recommendations_come_out_in_the_order_they_went_in() {
		let mut queue = Queue::new(vec![recommendation(1), recommendation(2)]);
		let mut seen = Vec::new();

		while let Ok(Some(recommendation)) = queue.next() {
			seen.push(recommendation.mbid.as_bytes()[0]);
		}

		assert_eq!(seen, vec![1, 2]);
	}
}
