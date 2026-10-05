use serde::Serialize;

use crate::declaration::Source;

use super::super::{recommendation::Recommendation, skip::Skip, stream::Stream};

#[derive(Serialize)]
pub(in crate::recommend) struct Pick {
	pub index: usize,
	pub turn: usize,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub artist: Vec<Source>,
	#[serde(flatten)]
	pub recommendation: Recommendation,
}

pub(in crate::recommend) fn collect(
	stream: &mut Stream,
	skip: &mut Skip,
	limit: usize,
) -> hmerr::Result<Vec<Pick>> {
	let mut pick = Vec::new();

	while pick.len() < limit {
		let Some(shown) = stream.next(skip)? else {
			break;
		};

		pick.push(Pick {
			index: shown.index,
			turn: shown.turn,
			artist: Vec::new(),
			recommendation: shown.recommendation,
		});
	}

	Ok(pick)
}

#[cfg(test)]
mod tests {
	use chrono::NaiveDate;

	use super::{
		super::super::{feed::canned, recommendation::Origin},
		*,
	};

	fn weekly(nibble: u8) -> Recommendation {
		Recommendation {
			mbid: Source::from_bytes([nibble; 16]),
			origin: Origin::WeeklyExploration {
				week: NaiveDate::default(),
				position: nibble.into(),
			},
		}
	}

	fn traced(feed: Vec<Vec<u8>>, limit: usize) -> Vec<Pick> {
		let mut stream = Stream::new(
			feed.into_iter()
				.map(|nibble| canned(nibble.into_iter().map(weekly).collect()))
				.collect(),
			false,
		);

		collect(&mut stream, &mut Skip::default(), limit).unwrap_or_default()
	}

	#[test]
	fn the_trace_stops_at_the_limit() {
		assert_eq!(traced(vec![vec![1, 2, 3, 4, 5]], 3).len(), 3);
	}

	#[test]
	fn a_limit_of_zero_traces_nothing() {
		assert!(traced(vec![vec![1, 2]], 0).is_empty());
	}

	#[test]
	fn the_trace_keeps_the_order_an_interactive_run_would_show() {
		let mbid: Vec<u8> = traced(vec![vec![1, 2], vec![4, 5]], 10)
			.iter()
			.map(|pick| pick.recommendation.mbid.as_bytes()[0])
			.collect();

		assert_eq!(mbid, vec![1, 4, 2, 5]);
	}

	#[test]
	fn a_pick_carries_the_turn_of_the_feed_it_came_from() {
		let turn: Vec<usize> = traced(vec![vec![1, 2], vec![4, 5]], 10)
			.iter()
			.map(|pick| pick.turn)
			.collect();

		assert_eq!(turn, vec![0, 1, 0, 1]);
	}
}
