mod absorb;
#[cfg(test)]
mod fixture;
mod louvain;
mod similarity;

pub(super) use similarity::{Similarity, similarity};

pub(super) fn detect(
	similarity: &Similarity,
	threshold: f64,
	resolution: f64,
	min_member: usize,
) -> Vec<usize> {
	let mut label = louvain::of(similarity.seed(), &similarity.edge(threshold), resolution);
	absorb::small(&mut label, similarity, min_member);

	label
}

#[cfg(test)]
mod tests {
	use super::{
		fixture::{group, seeded},
		*,
	};

	#[test]
	fn a_seed_with_no_listener_is_absorbed_by_its_nearest_neighbour() {
		let seed = seeded(&[&[1, 2, 3, 4], &[1, 2, 3, 4], &[1, 2], &[]]);
		let similarity = similarity(&seed, 5);
		let label = detect(&similarity, 0.15, 1.0, 3);

		assert_eq!(group(&label).len(), 1, "{label:?}");
	}

	#[test]
	fn a_seed_similar_to_nothing_joins_the_largest_community() {
		let seed = seeded(&[&[1, 2], &[1, 2], &[5, 6], &[5, 6], &[5, 6], &[]]);
		let similarity = similarity(&seed, 7);
		let label = detect(&similarity, 0.15, 1.0, 2);

		assert_eq!(label[5], label[2], "{label:?}");
		assert_ne!(label[5], label[0], "{label:?}");
	}
}
