use std::collections::HashSet;

use ansi::abbrev::{CYA, D, F, G, Y};

use crate::format::genre_list;

use super::{
	cohort::{self, Member},
	index::Index,
	partition::{self, Island, Terrain},
	rank,
	score::{self, Candidate},
	seed::Library,
};

const ROUND: usize = 3;

pub(super) struct Found {
	pub island: Vec<Island>,
	pub cohort: Vec<Vec<Member>>,
	pub candidate: Vec<Vec<Candidate>>,
}

impl Found {
	fn barren(&self) -> Vec<usize> {
		self.candidate
			.iter()
			.enumerate()
			.filter(|(_, candidate)| candidate.is_empty())
			.map(|(island, _)| island)
			.collect()
	}

	fn seed(&self, barren: &[usize]) -> Vec<usize> {
		barren
			.iter()
			.filter_map(|island| self.island.get(*island))
			.flat_map(|island| island.member.iter().copied())
			.collect()
	}

	pub(super) fn live(self) -> Self {
		let mut island = Vec::new();
		let mut cohort = Vec::new();
		let mut candidate = Vec::new();

		for ((one, member), served) in self
			.island
			.into_iter()
			.zip(self.cohort)
			.zip(self.candidate)
			.filter(|(_, candidate)| !candidate.is_empty())
		{
			island.push(one);
			cohort.push(member);
			candidate.push(served);
		}

		Self {
			island,
			cohort,
			candidate,
		}
	}
}

pub(super) fn raise(
	index: &Index,
	library: &Library,
	island: Vec<Island>,
	damp: f32,
) -> hmerr::Result<Found> {
	let cohort: Vec<Vec<Member>> = island
		.iter()
		.map(|island| cohort::of(library, island, cohort::SIZE))
		.collect();

	let (island, cohort) = rank::by_promise(island, cohort, library);
	let candidate = score::of(index, &cohort, damp)?;

	Ok(Found {
		island,
		cohort,
		candidate,
	})
}

pub(super) fn of(
	index: &Index,
	library: &Library,
	terrain: &Terrain,
	granularity: f64,
	damp: f32,
) -> hmerr::Result<Found> {
	let mut without: HashSet<usize> = HashSet::new();
	let mut round = 0;

	loop {
		let island = partition::of(terrain, granularity, &without);
		let found = raise(index, library, island, damp)?;
		let barren = found.barren();

		if barren.is_empty() {
			return Ok(found);
		}

		say(&found, &barren);

		let seed = found.seed(&barren);
		round += 1;

		if round == ROUND || seed.is_empty() || without.len() + seed.len() >= library.seed.len() {
			return Ok(found.live());
		}

		without.extend(seed);
		again(without.len());
	}
}

fn say(found: &Found, barren: &[usize]) {
	let named: Vec<(&Island, usize)> = barren
		.iter()
		.filter_map(|island| {
			Some((
				found.island.get(*island)?,
				found.cohort.get(*island).map_or(0, Vec::len),
			))
		})
		.collect();

	let width = named
		.iter()
		.map(|(island, _)| genre_list::width(&island.name))
		.max()
		.unwrap_or_default();

	for (island, user) in named {
		println!(
			"{name}{pad} {Y}no candidate{D} {CYA}{user:>4} {F}user{D} {G}{member:>4} {F}seed{D}",
			name = genre_list::text(&island.name),
			pad = genre_list::pad(&island.name, width),
			member = island.member.len(),
		);
	}
}

fn again(seed: usize) {
	println!("{F}re-partitioning without{D} {G}{seed}{D} {F}seed{D}\n");
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::declaration::Source;

	fn island(name: &str, member: &[usize]) -> Island {
		Island {
			name: name.to_string(),
			member: member.to_vec(),
		}
	}

	fn cohort(size: usize) -> Vec<Member> {
		(0..size)
			.map(|user| Member {
				user: i64::try_from(user).unwrap_or_default(),
				weight: 1.0,
			})
			.collect()
	}

	fn candidate(count: usize) -> Vec<Candidate> {
		(0..count)
			.map(|step| Candidate {
				mbid: Source::from_bytes([u8::try_from(step).unwrap_or_default(); 16]),
				score: 1.0,
				backer: 5,
				listener: 10,
				plays: 40,
			})
			.collect()
	}

	fn found(served: &[usize]) -> Found {
		Found {
			island: served
				.iter()
				.enumerate()
				.map(|(step, served)| island(&format!("isl{step}"), &[step, step + *served]))
				.collect(),
			cohort: served.iter().map(|served| cohort(served + 1)).collect(),
			candidate: served.iter().map(|served| candidate(*served)).collect(),
		}
	}

	fn name(found: &Found) -> Vec<&str> {
		found
			.island
			.iter()
			.map(|island| island.name.as_str())
			.collect()
	}

	#[test]
	fn an_island_with_no_candidate_is_barren() {
		assert_eq!(found(&[3, 0, 2, 0]).barren(), vec![1, 3]);
	}

	#[test]
	fn an_island_serving_everything_leaves_nothing_barren() {
		assert!(found(&[1, 2, 3]).barren().is_empty());
	}

	#[test]
	fn a_barren_island_hands_back_the_seeds_it_held() {
		let found = found(&[3, 0, 2]);

		assert_eq!(found.seed(&found.barren()), vec![1, 1]);
	}

	#[test]
	fn an_island_with_no_candidate_is_dropped_from_the_list() {
		assert_eq!(name(&found(&[3, 0, 2, 0]).live()), vec!["isl0", "isl2"]);
	}

	#[test]
	fn the_cohort_and_the_candidates_travel_with_the_island_they_belong_to() {
		let live = found(&[3, 0, 2, 0]).live();

		assert_eq!(
			live.cohort.iter().map(Vec::len).collect::<Vec<_>>(),
			vec![4, 3]
		);
		assert_eq!(
			live.candidate.iter().map(Vec::len).collect::<Vec<_>>(),
			vec![3, 2]
		);
	}

	#[test]
	fn a_list_where_every_island_serves_something_is_left_alone() {
		assert_eq!(
			name(&found(&[1, 2, 3]).live()),
			vec!["isl0", "isl1", "isl2"]
		);
	}

	#[test]
	fn a_list_where_no_island_serves_anything_comes_back_empty() {
		let live = found(&[0, 0]).live();

		assert!(live.island.is_empty());
		assert!(live.cohort.is_empty());
		assert!(live.candidate.is_empty());
	}
}
