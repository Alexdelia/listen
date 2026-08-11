use super::{partition::Island, real, seed::Library};

pub(super) const SIZE: usize = 500;

pub(super) struct Member {
	pub user: i64,
	pub weight: f32,
}

pub(super) fn of(library: &Library, island: &Island, size: usize) -> Vec<Member> {
	let mut affinity = vec![0.0f32; library.user.len()];
	let total = real::of(library.user.len());

	for member in &island.member {
		let Some(seed) = library.seed.get(*member) else {
			continue;
		};
		if seed.listener.is_empty() {
			continue;
		}

		let weight = seed.weight() * (total / real::of(seed.listener.len())).ln();
		if weight == 0.0 {
			continue;
		}

		for listener in &seed.deliberate {
			if let Some(affinity) = affinity.get_mut(*listener as usize) {
				*affinity += weight;
			}
		}
	}

	let mut member: Vec<Member> = affinity
		.into_iter()
		.enumerate()
		.filter(|(_, weight)| *weight > 0.0)
		.filter_map(|(user, weight)| {
			Some(Member {
				user: *library.user.get(user)?,
				weight,
			})
		})
		.collect();

	member.sort_unstable_by(|a, b| b.weight.total_cmp(&a.weight));
	member.truncate(size);

	member
}

#[cfg(test)]
mod tests {
	use super::{super::seed::Seed, *};
	use crate::declaration::Source;

	fn library(seed: &[(u8, &[u32])], user: usize) -> Library {
		Library {
			seed: seed
				.iter()
				.map(|(q, listener)| Seed {
					mbid: Source::from_bytes([*q; 16]),
					q: *q,
					listener: listener.to_vec(),
					deliberate: listener.to_vec(),
				})
				.collect(),
			user: (0..user).map(|user| user as i64).collect(),
			declared: Vec::new(),
		}
	}

	fn island(member: &[usize]) -> Island {
		Island {
			name: "test".to_string(),
			member: member.to_vec(),
		}
	}

	#[test]
	fn a_user_who_shares_a_liked_seed_joins_the_cohort() {
		let library = library(&[(4, &[0, 1])], 4);
		let cohort = of(&library, &island(&[0]), SIZE);

		assert_eq!(cohort.len(), 2);
	}

	#[test]
	fn a_user_who_shares_nothing_stays_out() {
		let library = library(&[(4, &[0])], 4);
		let cohort = of(&library, &island(&[0]), SIZE);

		assert_eq!(
			cohort.iter().map(|member| member.user).collect::<Vec<_>>(),
			vec![0]
		);
	}

	#[test]
	fn a_neutral_seed_recruits_nobody() {
		let library = library(&[(1, &[0, 1, 2])], 4);

		assert!(of(&library, &island(&[0]), SIZE).is_empty());
	}

	#[test]
	fn a_disliked_seed_cannot_pull_a_user_in() {
		let library = library(&[(0, &[0, 1])], 4);

		assert!(of(&library, &island(&[0]), SIZE).is_empty());
	}

	#[test]
	fn a_disliked_seed_pushes_a_user_out_of_a_cohort_it_would_have_joined() {
		let library = library(&[(2, &[0, 1]), (0, &[0])], 4);
		let cohort = of(&library, &island(&[0, 1]), SIZE);

		assert_eq!(
			cohort.iter().map(|member| member.user).collect::<Vec<_>>(),
			vec![1]
		);
	}

	#[test]
	fn the_rarest_seed_carries_the_most_affinity() {
		let library = library(&[(4, &[0]), (4, &[1, 2, 3])], 4);
		let cohort = of(&library, &island(&[0, 1]), SIZE);

		assert_eq!(cohort.first().map(|member| member.user), Some(0));
	}

	#[test]
	fn the_cohort_is_capped_at_its_size() {
		let library = library(&[(4, &[0, 1, 2, 3])], 8);

		assert_eq!(of(&library, &island(&[0]), 2).len(), 2);
	}

	#[test]
	fn the_cohort_comes_back_ordered_by_affinity() {
		let library = library(&[(4, &[0, 1]), (4, &[1])], 4);
		let cohort = of(&library, &island(&[0, 1]), SIZE);

		assert!(
			cohort
				.windows(2)
				.all(|pair| pair[0].weight >= pair[1].weight)
		);
	}

	#[test]
	fn an_empty_island_has_no_cohort() {
		let library = library(&[(4, &[0, 1])], 4);

		assert!(of(&library, &island(&[]), SIZE).is_empty());
	}
}
