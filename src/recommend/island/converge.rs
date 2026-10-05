use std::collections::HashSet;

use ansi::abbrev::{CYA, D, F, G, Y};
use serde::Serialize;

use crate::format::genre_list;

use super::{
	cohort::{self, Member},
	index::Index,
	partition::{self, Island, Terrain},
	rank,
	score::{self, Candidate, Tuning},
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

#[derive(Serialize)]
pub(super) struct Glance {
	pub name: String,
	pub seed: usize,
	pub user: usize,
	pub candidate: usize,
}

#[derive(Serialize)]
pub(super) struct Round {
	pub island: Vec<Glance>,
	pub next_without: Option<usize>,
}

impl Round {
	fn of(found: &Found, next_without: Option<usize>) -> Self {
		Self {
			island: found
				.island
				.iter()
				.zip(&found.cohort)
				.zip(&found.candidate)
				.map(|((island, cohort), candidate)| Glance {
					name: island.name.clone(),
					seed: island.member.len(),
					user: cohort.len(),
					candidate: candidate.len(),
				})
				.collect(),
			next_without,
		}
	}
}

pub(super) struct Converged {
	pub found: Found,
	pub round: Vec<Round>,
}

pub(super) fn narrowed(found: Found) -> Converged {
	let round = vec![Round::of(&found, None)];

	Converged {
		found: found.live(),
		round,
	}
}

pub(super) fn raise(
	index: &Index,
	library: &Library,
	island: Vec<Island>,
	tuning: Tuning,
) -> hmerr::Result<Found> {
	let cohort: Vec<Vec<Member>> = island
		.iter()
		.map(|island| cohort::of(library, island, cohort::SIZE))
		.collect();

	let (island, cohort) = rank::by_promise(island, cohort, library);
	let candidate = score::of(index, &cohort, tuning)?;

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
	tuning: Tuning,
) -> hmerr::Result<Converged> {
	let mut without: HashSet<usize> = HashSet::new();
	let mut round = Vec::new();

	loop {
		let island = partition::of(terrain, granularity, &without);
		let found = raise(index, library, island, tuning)?;
		let barren = found.barren();

		if barren.is_empty() {
			round.push(Round::of(&found, None));
			return Ok(Converged { found, round });
		}

		let seed = found.seed(&barren);
		let last = round.len() + 1 == ROUND
			|| seed.is_empty()
			|| without.len() + seed.len() >= library.seed.len();

		if last {
			round.push(Round::of(&found, None));
			return Ok(Converged {
				found: found.live(),
				round,
			});
		}

		without.extend(seed);
		round.push(Round::of(&found, Some(without.len())));
	}
}

pub(super) fn print(round: &[Round]) {
	for round in round {
		let barren: Vec<&Glance> = round
			.island
			.iter()
			.filter(|glance| glance.candidate == 0)
			.collect();

		say(&barren);

		if let Some(without) = round.next_without {
			again(without);
		}
	}
}

fn say(barren: &[&Glance]) {
	let width = barren
		.iter()
		.map(|glance| genre_list::width(&glance.name))
		.max()
		.unwrap_or_default();

	for glance in barren {
		println!(
			"{name}{pad} {Y}no candidate{D} {CYA}{user:>4} {F}user{D} {G}{member:>4} {F}seed{D}",
			name = genre_list::text(&glance.name),
			pad = genre_list::pad(&glance.name, width),
			user = glance.user,
			member = glance.seed,
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
				liked_seed: 1,
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

	#[test]
	fn a_round_tells_how_many_candidates_each_island_found() {
		let round = Round::of(&found(&[0, 3]), None);

		let candidate: Vec<usize> = round.island.iter().map(|glance| glance.candidate).collect();

		assert_eq!(candidate, vec![0, 3]);
		assert_eq!(round.next_without, None);
	}

	#[test]
	fn a_narrowed_run_records_one_round_and_keeps_only_islands_with_candidates() {
		let converged = narrowed(found(&[0, 3]));

		assert_eq!(converged.round.len(), 1);
		assert_eq!(converged.round[0].island.len(), 2);
		assert_eq!(converged.found.island.len(), 1);
	}
}
