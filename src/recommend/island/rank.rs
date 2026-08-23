use super::{
	cohort::{self, Member},
	partition::Island,
	real,
	seed::Library,
};

pub(super) fn by_promise(
	island: Vec<Island>,
	cohort: Vec<Vec<Member>>,
	library: &Library,
) -> (Vec<Island>, Vec<Vec<Member>>) {
	let mut ranked: Vec<(f32, Island, Vec<Member>)> = island
		.into_iter()
		.zip(cohort)
		.map(|(island, cohort)| (promise(&island, cohort.len(), library), island, cohort))
		.collect();

	ranked.sort_by(|a, b| b.0.total_cmp(&a.0));

	ranked
		.into_iter()
		.map(|(_, island, cohort)| (island, cohort))
		.unzip()
}

pub(super) fn promise(island: &Island, cohort: usize, library: &Library) -> f32 {
	let support = real::of(cohort);
	let full_cohort = real::of(cohort::SIZE);

	(island.q(&library.seed) * support + library.q() * full_cohort) / (support + full_cohort)
}

#[cfg(test)]
mod tests {
	use super::{
		super::seed::Seed,
		cohort::{Member, SIZE},
		*,
	};
	use crate::declaration::{Q, Source};

	fn library(q: &[Q]) -> Library {
		Library {
			seed: q
				.iter()
				.map(|q| Seed {
					mbid: Source::from_bytes([*q; 16]),
					q: *q,
					listener: Vec::new(),
				})
				.collect(),
			user: Vec::new(),
			declared: Vec::new(),
		}
	}

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

	fn name(island: &[Island]) -> Vec<&str> {
		island.iter().map(|island| island.name.as_str()).collect()
	}

	const TINY: [usize; 2] = [0, 1];
	const WIDE: [usize; 8] = [2, 3, 4, 5, 6, 7, 8, 9];

	fn well_rated_tiny_island_in_a_lesser_library() -> Library {
		let mut q = vec![3; TINY.len()];
		q.extend(std::iter::repeat_n(2, WIDE.len()));
		q.extend(std::iter::repeat_n(1, 10));

		library(&q)
	}

	#[test]
	fn a_small_cohort_cannot_outrank_a_full_one_on_a_handful_of_ratings() {
		let library = well_rated_tiny_island_in_a_lesser_library();
		let (ranked, _) = by_promise(
			vec![island("tiny", &TINY), island("wide", &WIDE)],
			vec![cohort(35), cohort(SIZE)],
			&library,
		);

		assert_eq!(name(&ranked), vec!["wide", "tiny"]);
	}

	#[test]
	fn the_same_island_ranks_higher_once_its_cohort_fills_up() {
		let library = well_rated_tiny_island_in_a_lesser_library();
		let tiny = island("tiny", &TINY);

		assert!(promise(&tiny, SIZE, &library) > promise(&tiny, 35, &library));
	}

	#[test]
	fn a_full_cohort_keeps_the_island_its_q_earned() {
		let library = library(&[4, 4, 1, 1]);
		let (ranked, _) = by_promise(
			vec![island("liked", &[0, 1]), island("meh", &[2, 3])],
			vec![cohort(SIZE), cohort(SIZE)],
			&library,
		);

		assert_eq!(name(&ranked), vec!["liked", "meh"]);
	}

	#[test]
	fn an_island_without_a_cohort_falls_back_to_the_library_mean() {
		let library = library(&[4, 4, 0, 0]);
		let promise = promise(&island("orphan", &[0, 1]), 0, &library);

		assert!((promise - library.q()).abs() < f32::EPSILON, "{promise}");
	}

	#[test]
	fn the_cohort_travels_with_the_island_it_belongs_to() {
		let library = well_rated_tiny_island_in_a_lesser_library();
		let (ranked, cohort) = by_promise(
			vec![island("tiny", &TINY), island("wide", &WIDE)],
			vec![cohort(1), cohort(SIZE)],
			&library,
		);

		assert_eq!(name(&ranked), vec!["wide", "tiny"]);
		assert_eq!(
			cohort.iter().map(Vec::len).collect::<Vec<_>>(),
			vec![SIZE, 1]
		);
	}
}
