mod cluster;
mod genre;
mod name;
mod requested;

use std::collections::HashSet;

use crate::declaration::Source;

use super::seed::{Library, Seed, mean_q};

const THRESHOLD: f64 = 0.15;
const MIN_MEMBER: usize = 10;

pub(super) struct Request {
	pub recording: Vec<Source>,
	pub genre: Vec<String>,
}

impl Request {
	pub(super) const fn asked(&self) -> bool {
		!self.recording.is_empty() || !self.genre.is_empty()
	}
}

pub(super) struct Island {
	pub name: String,
	pub member: Vec<usize>,
}

impl Island {
	pub(super) fn q(&self, seed: &[Seed]) -> f32 {
		mean_q(self.member.iter().filter_map(|member| seed.get(*member)))
	}
}

pub(super) struct Terrain {
	genre: Vec<Vec<String>>,
	similarity: cluster::Similarity,
}

pub(super) fn terrain(library: &Library) -> Terrain {
	Terrain {
		genre: read_genre(library),
		similarity: cluster::similarity(&library.seed, library.user.len()),
	}
}

pub(super) fn of(terrain: &Terrain, granularity: f64, without: &HashSet<usize>) -> Vec<Island> {
	let kept: Vec<usize> = (0..terrain.genre.len())
		.filter(|seed| !without.contains(seed))
		.collect();

	let label = cluster::detect(
		&terrain.similarity.restrict(&kept),
		THRESHOLD,
		granularity,
		MIN_MEMBER,
	);

	let detected = label.iter().copied().max().map_or(0, |label| label + 1);

	let mut member: Vec<Vec<usize>> = vec![Vec::new(); detected];
	for (node, label) in label.iter().enumerate() {
		if let Some(seed) = kept.get(node) {
			member[*label].push(*seed);
		}
	}
	member.retain(|member| !member.is_empty());

	name::name(&terrain.genre, &member)
		.into_iter()
		.zip(member)
		.map(|(name, member)| Island { name, member })
		.collect()
}

pub(super) fn requested(library: &Library, request: &Request) -> hmerr::Result<Vec<Island>> {
	let (name, member) = requested::island(library, &read_genre(library), request)?;

	Ok(vec![Island { name, member }])
}

fn read_genre(library: &Library) -> Vec<Vec<String>> {
	library
		.seed
		.iter()
		.map(|seed| genre::read(seed.mbid))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recommend::island::seed::{Listener, Seed};

	fn request(recording: usize, genre: usize) -> Request {
		Request {
			recording: (0..recording)
				.map(|byte| Source::from_bytes([u8::try_from(byte).unwrap_or_default(); 16]))
				.collect(),
			genre: (0..genre).map(|token| token.to_string()).collect(),
		}
	}

	#[test]
	fn no_flag_is_no_request() {
		assert!(!request(0, 0).asked());
		assert!(request(1, 0).asked());
		assert!(request(0, 1).asked());
		assert!(request(1, 1).asked());
	}

	fn library(listener: &[&[u32]], user: usize) -> Library {
		Library {
			seed: listener
				.iter()
				.enumerate()
				.map(|(index, listener)| Seed {
					mbid: Source::from_bytes([u8::try_from(index).unwrap_or_default(); 16]),
					q: 3,
					listener: listener
						.iter()
						.map(|user| Listener {
							user: *user,
							weight: 1.0,
						})
						.collect(),
				})
				.collect(),
			user: (0..i64::try_from(user).unwrap_or_default()).collect(),
			declared: Vec::new(),
		}
	}

	fn two_crowds() -> Library {
		library(
			&[
				&[0, 1, 2],
				&[0, 1, 2],
				&[0, 1, 2],
				&[7, 8, 9],
				&[7, 8, 9],
				&[7, 8, 9],
			],
			10,
		)
	}

	fn member(island: &[Island]) -> Vec<usize> {
		let mut member: Vec<usize> = island
			.iter()
			.flat_map(|island| island.member.iter().copied())
			.collect();
		member.sort_unstable();

		member
	}

	#[test]
	fn every_seed_lands_in_an_island_when_none_is_left_out() {
		let library = two_crowds();
		let island = of(&terrain(&library), 1.0, &HashSet::new());

		assert_eq!(member(&island), vec![0, 1, 2, 3, 4, 5]);
	}

	#[test]
	fn a_seed_left_out_lands_in_no_island() {
		let library = two_crowds();
		let island = of(&terrain(&library), 1.0, &HashSet::from([3, 4, 5]));

		assert_eq!(member(&island), vec![0, 1, 2]);
	}

	#[test]
	fn a_member_left_over_still_indexes_the_whole_library() {
		let library = two_crowds();
		let island = of(&terrain(&library), 1.0, &HashSet::from([0]));

		assert!(member(&island).iter().all(|member| *member != 0));
		assert!(island.iter().all(|island| {
			island
				.member
				.iter()
				.all(|member| library.seed.get(*member).is_some_and(|seed| seed.q == 3))
		}));
	}

	#[test]
	fn leaving_every_seed_out_detects_no_island() {
		let library = two_crowds();
		let without = (0..library.seed.len()).collect();

		assert!(of(&terrain(&library), 1.0, &without).is_empty());
	}
}
