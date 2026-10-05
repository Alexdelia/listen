use std::cmp::Ordering;

use ansi::abbrev::{CYA, D, F, G, M, Y};
use listen_index as index;
use serde::Serialize;

use crate::{
	declaration::Q,
	format::{self, genre_list, human_readable_number},
};

use super::{
	super::labelled::Labelled,
	cohort,
	converge::{self, Found, Round},
	rank, score, seed,
};

const TOP: usize = 20;

#[derive(Serialize)]
pub(in crate::recommend) struct Report {
	#[serde(skip)]
	index: index::Meta,
	library: Library,
	tuning: Tuning,
	mode: Mode,
	round: Vec<Round>,
	island: Vec<Island>,
}

#[derive(Serialize)]
struct Library {
	seed: usize,
	user: usize,
	unsupported: usize,
	q: f32,
}

#[derive(Serialize)]
struct Tuning {
	popularity_damp: f32,
	allow_known_artist: bool,
	backing: score::Backing,
	granularity: f64,
	min_backer: u32,
	min_distinct_backer: u32,
	per_island: usize,
	cohort_size: usize,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Mode {
	Detected,
	Requested,
	Pinned,
}

#[derive(Serialize)]
struct Island {
	name: String,
	promise: f32,
	q: f32,
	candidate: usize,
	seed: Vec<Seed>,
	cohort: Cohort,
}

#[derive(Serialize)]
struct Seed {
	#[serde(flatten)]
	recording: Labelled,
	q: Q,
	listener: usize,
}

#[derive(Serialize)]
struct Cohort {
	size: usize,
	weight: Option<Spread<f32>>,
	liked_seed: Option<Spread<u32>>,
	top: Vec<Member>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Spread<T> {
	min: T,
	median: T,
	max: T,
}

#[derive(Serialize)]
struct Member {
	user: i64,
	weight: f32,
	liked_seed: u32,
}

pub(super) struct Run<'a> {
	pub meta: &'a index::Meta,
	pub library: &'a seed::Library,
	pub tuning: score::Tuning,
	pub granularity: f64,
	pub mode: Mode,
}

pub(super) fn of(run: &Run<'_>, found: &Found, round: Vec<Round>) -> Report {
	let library = run.library;

	Report {
		index: run.meta.clone(),
		library: Library {
			seed: library.seed.len(),
			user: library.user.len(),
			unsupported: library.unsupported(),
			q: library.q(),
		},
		tuning: Tuning {
			popularity_damp: run.tuning.damp,
			allow_known_artist: run.tuning.allow_known_artist,
			backing: run.tuning.backing,
			granularity: run.granularity,
			min_backer: score::MIN_BACKER,
			min_distinct_backer: score::MIN_DISTINCT_BACKER,
			per_island: score::PER_ISLAND,
			cohort_size: cohort::SIZE,
		},
		mode: run.mode,
		round,
		island: found
			.island
			.iter()
			.zip(&found.cohort)
			.zip(&found.candidate)
			.map(|((island, member), candidate)| Island {
				name: island.name.clone(),
				promise: rank::promise(island, member.len(), library),
				q: island.q(&library.seed),
				candidate: candidate.len(),
				seed: island
					.member
					.iter()
					.filter_map(|member| library.seed.get(*member))
					.map(|seed| Seed {
						recording: Labelled(seed.mbid),
						q: seed.q,
						listener: seed.listener.len(),
					})
					.collect(),
				cohort: Cohort::of(member),
			})
			.collect(),
	}
}

impl Cohort {
	fn of(member: &[cohort::Member]) -> Self {
		Self {
			size: member.len(),
			weight: spread(
				member.iter().map(|member| member.weight).collect(),
				f32::total_cmp,
			),
			liked_seed: spread(
				member.iter().map(|member| member.liked_seed).collect(),
				Ord::cmp,
			),
			top: member
				.iter()
				.take(TOP)
				.map(|member| Member {
					user: member.user,
					weight: member.weight,
					liked_seed: member.liked_seed,
				})
				.collect(),
		}
	}
}

fn spread<T: Copy>(mut value: Vec<T>, order: impl Fn(&T, &T) -> Ordering) -> Option<Spread<T>> {
	value.sort_by(order);
	let middle = value.len().checked_sub(1)? / 2;

	Some(Spread {
		min: *value.first()?,
		median: *value.get(middle)?,
		max: *value.last()?,
	})
}

impl Report {
	pub(in crate::recommend) fn print(&self) {
		self.print_index();
		converge::print(&self.round);
		self.describe();
	}

	fn print_index(&self) {
		let meta = &self.index;

		println!(
			"index {CYA}{built}{D}: {G}{recording} {G}{F}recording{D} {M}{listen} {M}{F}listen{D} {CYA}{user} {F}user{D}",
			built = meta.built,
			recording = human_readable_number::text(meta.recording),
			listen = human_readable_number::text(meta.user_listen),
			user = human_readable_number::text(meta.user),
		);

		if meta.absorbed > 0 {
			println!(
				"covered to {CYA}{covered}{D} {F}after{D} {CYA}{absorbed}{D} {F}incremental{D}",
				covered = day(meta.covered()),
				absorbed = meta.absorbed
			);
		}

		for gap in &meta.gap {
			println!(
				"{Y}gap{D} {F}from{D} {CYA}{from}{D} {F}to{D} {CYA}{to}{D}",
				from = day(&gap.from),
				to = day(&gap.to)
			);
		}

		let unsupported = self.library.unsupported;
		if unsupported > 0 {
			println!(
				"{unsupported}{F}/{D}{declared} {F}declared recording have no listener in the index{D}",
				declared = self.library.seed + unsupported,
			);
		}

		println!();
	}

	fn describe(&self) {
		if self.island.is_empty() {
			println!("{Y}no island has a candidate{D}");

			return;
		}

		let width = self
			.island
			.iter()
			.map(|island| genre_list::width(&island.name))
			.max()
			.unwrap_or_default();

		for island in &self.island {
			println!(
				"{name}{pad} {Y}{promise:.2} {F}promise  {q_color}{q:.2}{D} {CYA}{size:>4} {F}user{D} {G}{member:>4} {F}seed{D}",
				name = genre_list::text(&island.name),
				pad = genre_list::pad(&island.name, width),
				promise = island.promise,
				q_color = format::q_f32_color(island.q),
				q = island.q,
				member = island.seed.len(),
				size = island.cohort.size,
			);
		}
	}
}

fn day(timestamp: &str) -> &str {
	timestamp.split(' ').next().unwrap_or(timestamp)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn member(weight: &[f32]) -> Vec<cohort::Member> {
		weight
			.iter()
			.zip(0..)
			.map(|(weight, user)| cohort::Member {
				user,
				weight: *weight,
				liked_seed: u32::try_from(user).unwrap_or_default() + 1,
			})
			.collect()
	}

	#[test]
	fn the_cohort_summary_keeps_only_the_heaviest_members() {
		let weight: Vec<f32> = (0..25u8).rev().map(f32::from).collect();
		let cohort = Cohort::of(&member(&weight));

		assert_eq!(cohort.size, 25);
		assert_eq!(cohort.top.len(), TOP);
		assert!((cohort.top[0].weight - 24.0).abs() < f32::EPSILON);
	}

	#[test]
	fn the_median_of_an_even_count_is_the_lower_middle() {
		assert_eq!(
			spread(vec![4u32, 1, 3, 2], Ord::cmp),
			Some(Spread {
				min: 1,
				median: 2,
				max: 4
			})
		);
	}

	#[test]
	fn the_median_of_an_odd_count_is_the_middle() {
		assert_eq!(
			spread(vec![5u32, 1, 3], Ord::cmp),
			Some(Spread {
				min: 1,
				median: 3,
				max: 5
			})
		);
	}

	#[test]
	fn an_empty_cohort_has_no_spread() {
		let cohort = Cohort::of(&[]);

		assert_eq!(cohort.size, 0);
		assert!(cohort.weight.is_none() && cohort.liked_seed.is_none());
	}
}
