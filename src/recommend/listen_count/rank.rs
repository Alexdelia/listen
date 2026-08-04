use std::cmp::Reverse;

use crate::recommend::recommendation::{Origin, Recommendation};

use super::payload::Popularity;

pub(super) fn rank(popularity: Vec<Popularity>) -> Vec<Recommendation> {
	let mut listened = popularity
		.into_iter()
		.filter_map(|entry| {
			Some((
				entry.recording_mbid,
				entry.total_listen_count.filter(|listen| *listen > 0)?,
				entry.total_user_count.unwrap_or_default(),
			))
		})
		.collect::<Vec<_>>();

	listened.sort_unstable_by_key(|(_, listen, _)| Reverse(*listen));

	listened
		.into_iter()
		.zip(0..)
		.map(|((mbid, listen, user), position)| Recommendation {
			mbid,
			origin: Origin::ListenCount {
				listen,
				user,
				position,
			},
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::declaration::Source;

	fn popularity(nibble: u8, listen: Option<u64>) -> Popularity {
		Popularity {
			recording_mbid: Source::from_bytes([nibble; 16]),
			total_listen_count: listen,
			total_user_count: listen.map(|listen| listen / 10),
		}
	}

	fn nibbles(ranked: &[Recommendation]) -> Vec<u8> {
		ranked
			.iter()
			.map(|recommendation| recommendation.mbid.as_bytes()[0])
			.collect()
	}

	#[test]
	fn the_most_listened_recording_comes_first() {
		let ranked = rank(vec![
			popularity(1, Some(700)),
			popularity(2, Some(1200)),
			popularity(3, Some(900)),
		]);

		assert_eq!(nibbles(&ranked), vec![2, 3, 1]);
	}

	#[test]
	fn a_recording_nobody_listened_to_is_dropped() {
		let ranked = rank(vec![
			popularity(1, Some(700)),
			popularity(2, None),
			popularity(3, Some(0)),
		]);

		assert_eq!(nibbles(&ranked), vec![1]);
	}

	#[test]
	fn the_position_follows_the_ranking() {
		let ranked = rank(vec![popularity(1, Some(700)), popularity(2, Some(1200))]);

		assert_eq!(
			ranked
				.iter()
				.map(|recommendation| recommendation.origin.position())
				.collect::<Vec<_>>(),
			vec![0, 1]
		);
	}

	#[test]
	fn the_counts_travel_with_the_recommendation() {
		let ranked = rank(vec![popularity(1, Some(700))]);

		assert!(matches!(
			ranked.first().map(|recommendation| &recommendation.origin),
			Some(Origin::ListenCount {
				listen: 700,
				user: 70,
				..
			})
		));
	}
}
