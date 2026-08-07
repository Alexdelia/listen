use std::{cmp::Reverse, collections::HashMap};

use chrono::NaiveDate;

use crate::{
	args::RecommendSort,
	declaration::Source,
	recommend::recommendation::{Origin, Recommendation},
};

use super::{catalogue::Catalogue, payload::Popularity};

struct Listened {
	mbid: Source,
	listen: u64,
	user: u64,
	released: Option<NaiveDate>,
}

pub(super) fn rank(
	sort: RecommendSort,
	catalogue: &Catalogue,
	popularity: Vec<Popularity>,
) -> Vec<Recommendation> {
	let count = popularity
		.into_iter()
		.map(|entry| (entry.recording_mbid, entry))
		.collect::<HashMap<_, _>>();

	let mut listened = catalogue
		.recording
		.iter()
		.map(|mbid| {
			let found = count.get(mbid);

			Listened {
				mbid: *mbid,
				listen: found
					.and_then(|found| found.total_listen_count)
					.unwrap_or_default(),
				user: found
					.and_then(|found| found.total_user_count)
					.unwrap_or_default(),
				released: catalogue.released.get(mbid).copied(),
			}
		})
		.collect::<Vec<_>>();

	match sort {
		RecommendSort::Popularity => {
			listened.retain(|entry| entry.listen > 0);
			listened.sort_unstable_by_key(most_listened);
		}
		RecommendSort::Newest => listened.sort_unstable_by_key(newest_released),
	}

	listened
		.into_iter()
		.zip(0..)
		.map(|(entry, position)| Recommendation {
			mbid: entry.mbid,
			origin: Origin::ListenCount {
				listen: entry.listen,
				user: entry.user,
				released: entry.released,
				position,
			},
		})
		.collect()
}

fn most_listened(entry: &Listened) -> Reverse<u64> {
	Reverse(entry.listen)
}

fn newest_released(entry: &Listened) -> (Reverse<Option<NaiveDate>>, Reverse<u64>) {
	(Reverse(entry.released), Reverse(entry.listen))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn popularity(nibble: u8, listen: Option<u64>) -> Popularity {
		Popularity {
			recording_mbid: mbid(nibble),
			total_listen_count: listen,
			total_user_count: listen.map(|listen| listen / 10),
		}
	}

	fn mbid(nibble: u8) -> Source {
		Source::from_bytes([nibble; 16])
	}

	fn catalogue(nibble: &[u8], dated: &[(u8, (i32, u32, u32))]) -> Catalogue {
		Catalogue {
			artist: "Mili".to_string(),
			recording: nibble.iter().map(|nibble| mbid(*nibble)).collect(),
			released: dated
				.iter()
				.filter_map(|(nibble, (year, month, day))| {
					Some((mbid(*nibble), NaiveDate::from_ymd_opt(*year, *month, *day)?))
				})
				.collect(),
		}
	}

	fn nibbles(ranked: &[Recommendation]) -> Vec<u8> {
		ranked
			.iter()
			.map(|recommendation| recommendation.mbid.as_bytes()[0])
			.collect()
	}

	fn by_popularity(nibble: &[u8], popularity: Vec<Popularity>) -> Vec<Recommendation> {
		rank(
			RecommendSort::Popularity,
			&catalogue(nibble, &[]),
			popularity,
		)
	}

	#[test]
	fn the_most_listened_recording_comes_first() {
		let ranked = by_popularity(
			&[1, 2, 3],
			vec![
				popularity(1, Some(700)),
				popularity(2, Some(1200)),
				popularity(3, Some(900)),
			],
		);

		assert_eq!(nibbles(&ranked), vec![2, 3, 1]);
	}

	#[test]
	fn a_recording_nobody_listened_to_is_dropped() {
		let ranked = by_popularity(
			&[1, 2, 3],
			vec![
				popularity(1, Some(700)),
				popularity(2, None),
				popularity(3, Some(0)),
			],
		);

		assert_eq!(nibbles(&ranked), vec![1]);
	}

	#[test]
	fn a_recording_the_popularity_left_out_is_dropped() {
		let ranked = by_popularity(&[1, 2], vec![popularity(1, Some(700))]);

		assert_eq!(nibbles(&ranked), vec![1]);
	}

	#[test]
	fn the_position_follows_the_ranking() {
		let ranked = by_popularity(
			&[1, 2],
			vec![popularity(1, Some(700)), popularity(2, Some(1200))],
		);

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
		let ranked = by_popularity(&[1], vec![popularity(1, Some(700))]);

		assert!(matches!(
			ranked.first().map(|recommendation| &recommendation.origin),
			Some(Origin::ListenCount {
				listen: 700,
				user: 70,
				..
			})
		));
	}

	#[test]
	fn the_newest_release_comes_first() {
		let ranked = rank(
			RecommendSort::Newest,
			&catalogue(
				&[1, 2, 3],
				&[(1, (2019, 3, 8)), (2, (2026, 1, 17)), (3, (2021, 11, 4))],
			),
			vec![
				popularity(1, Some(1200)),
				popularity(2, Some(700)),
				popularity(3, Some(900)),
			],
		);

		assert_eq!(nibbles(&ranked), vec![2, 3, 1]);
	}

	#[test]
	fn a_just_released_recording_nobody_listened_to_yet_still_comes_first() {
		let ranked = rank(
			RecommendSort::Newest,
			&catalogue(&[1, 2], &[(1, (2019, 3, 8)), (2, (2026, 8, 3))]),
			vec![popularity(1, Some(1200)), popularity(2, None)],
		);

		assert_eq!(nibbles(&ranked), vec![2, 1]);
	}

	#[test]
	fn a_recording_the_popularity_left_out_is_kept() {
		let ranked = rank(
			RecommendSort::Newest,
			&catalogue(&[1, 2], &[(1, (2019, 3, 8)), (2, (2026, 8, 3))]),
			vec![popularity(1, Some(1200))],
		);

		assert_eq!(nibbles(&ranked), vec![2, 1]);
	}

	#[test]
	fn an_undated_recording_sits_after_every_dated_one() {
		let ranked = rank(
			RecommendSort::Newest,
			&catalogue(&[1, 2, 3], &[(3, (1998, 6, 2))]),
			vec![
				popularity(1, Some(700)),
				popularity(2, Some(1200)),
				popularity(3, Some(900)),
			],
		);

		assert_eq!(nibbles(&ranked), vec![3, 2, 1]);
	}

	#[test]
	fn the_release_date_travels_with_the_recommendation() {
		let ranked = rank(
			RecommendSort::Newest,
			&catalogue(&[1], &[(1, (2019, 3, 8))]),
			vec![popularity(1, Some(700))],
		);

		assert!(matches!(
			ranked.first().map(|recommendation| &recommendation.origin),
			Some(Origin::ListenCount { released, .. })
				if *released == NaiveDate::from_ymd_opt(2019, 3, 8)
		));
	}
}
