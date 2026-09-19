use super::super::super::{real, seed::Seed};

pub(in crate::recommend::island::partition) struct Similarity {
	value: Vec<f64>,
	seed: usize,
}

impl Similarity {
	pub(super) fn of(&self, a: usize, b: usize) -> f64 {
		self.value[a * self.seed + b]
	}

	pub(super) const fn seed(&self) -> usize {
		self.seed
	}

	pub(in crate::recommend::island::partition) fn restrict(&self, keep: &[usize]) -> Self {
		let seed = keep.len();
		let mut value = vec![0.0; seed * seed];

		for (a, from_a) in keep.iter().enumerate() {
			for (b, from_b) in keep.iter().enumerate() {
				value[a * seed + b] = self.of(*from_a, *from_b);
			}
		}

		Self { value, seed }
	}

	pub(super) fn edge(&self, threshold: f64) -> Vec<(usize, usize, f64)> {
		let mut edge = Vec::new();

		for a in 0..self.seed {
			for b in (a + 1)..self.seed {
				let weight = self.of(a, b);
				if weight >= threshold {
					edge.push((a, b, weight));
				}
			}
		}

		edge
	}
}

pub(in crate::recommend::island::partition) fn similarity(
	seed: &[Seed],
	user: usize,
) -> Similarity {
	let word = user.div_ceil(u64::BITS as usize).max(1);
	let mut bit = vec![0u64; seed.len() * word];

	for (index, seed) in seed.iter().enumerate() {
		for listener in &seed.listener {
			let listener = listener.user as usize;
			bit[index * word + listener / 64] |= 1 << (listener % 64);
		}
	}

	let mut value = vec![0.0; seed.len() * seed.len()];

	for a in 0..seed.len() {
		let size_a = seed[a].listener.len();
		if size_a == 0 {
			continue;
		}

		for b in (a + 1)..seed.len() {
			let size_b = seed[b].listener.len();
			if size_b == 0 {
				continue;
			}

			let shared: u32 = (0..word)
				.map(|w| (bit[a * word + w] & bit[b * word + w]).count_ones())
				.sum();
			if shared == 0 {
				continue;
			}

			let cosine = f64::from(shared) / (real::wide(size_a) * real::wide(size_b)).sqrt();

			value[a * seed.len() + b] = cosine;
			value[b * seed.len() + a] = cosine;
		}
	}

	Similarity {
		value,
		seed: seed.len(),
	}
}

#[cfg(test)]
mod tests {
	use super::{super::fixture::seeded, *};

	#[test]
	fn identical_audiences_are_perfectly_similar() {
		let seed = seeded(&[&[1, 2, 3], &[1, 2, 3]]);
		let similarity = similarity(&seed, 4);

		assert!((similarity.of(0, 1) - 1.0).abs() < 1e-6);
	}

	#[test]
	fn disjoint_audiences_are_not_similar_at_all() {
		let seed = seeded(&[&[1, 2], &[3, 4]]);
		let similarity = similarity(&seed, 5);

		assert!(similarity.of(0, 1).abs() < f64::EPSILON);
	}

	#[test]
	fn a_restricted_matrix_holds_only_the_seeds_it_kept() {
		let seed = seeded(&[&[1, 2, 3], &[1, 2, 3], &[9], &[1, 2, 3]]);
		let similarity = similarity(&seed, 10).restrict(&[0, 3]);

		assert_eq!(similarity.seed(), 2);
	}

	#[test]
	fn a_restricted_matrix_keeps_what_the_seeds_it_kept_were_worth_to_each_other() {
		let seed = seeded(&[&[1, 2, 3], &[9], &[2, 3, 4]]);
		let full = similarity(&seed, 10);
		let restricted = full.restrict(&[0, 2]);

		assert!((restricted.of(0, 1) - full.of(0, 2)).abs() < f64::EPSILON);
		assert!((restricted.of(1, 0) - full.of(2, 0)).abs() < f64::EPSILON);
	}

	#[test]
	fn a_seed_left_out_takes_its_edges_with_it() {
		let seed = seeded(&[&[1, 2], &[1, 2], &[1, 2]]);
		let restricted = similarity(&seed, 4).restrict(&[0, 2]);

		assert_eq!(restricted.edge(0.15).len(), 1);
	}

	#[test]
	fn similarity_is_symmetric() {
		let seed = seeded(&[&[1, 2, 3], &[2, 3, 4], &[9]]);
		let similarity = similarity(&seed, 10);

		for a in 0..3 {
			for b in 0..3 {
				assert!((similarity.of(a, b) - similarity.of(b, a)).abs() < f64::EPSILON);
			}
		}
	}
}
