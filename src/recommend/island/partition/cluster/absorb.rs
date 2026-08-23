use std::{cmp::Reverse, collections::HashMap};

use super::Similarity;

pub(super) fn small(label: &mut [usize], similarity: &Similarity, min_member: usize) {
	loop {
		let mut size: HashMap<usize, usize> = HashMap::new();
		for community in label.iter() {
			*size.entry(*community).or_default() += 1;
		}

		if size.len() <= 1 {
			return;
		}

		let Some(smallest) = size
			.iter()
			.filter(|(_, count)| **count < min_member)
			.min_by_key(|(community, count)| (**count, **community))
			.map(|(community, _)| *community)
		else {
			return;
		};

		let member: Vec<usize> = (0..label.len())
			.filter(|node| label[*node] == smallest)
			.collect();

		let mut moved = false;
		for node in member {
			let host =
				nearest(node, label, similarity, smallest).or_else(|| largest(&size, smallest));

			if let Some(community) = host {
				label[node] = community;
				moved = true;
			}
		}

		if !moved {
			return;
		}
	}
}

fn nearest(
	node: usize,
	label: &[usize],
	similarity: &Similarity,
	smallest: usize,
) -> Option<usize> {
	let mut best: Option<(usize, f64)> = None;

	for (peer, community) in label.iter().enumerate() {
		if *community == smallest {
			continue;
		}

		let weight = similarity.of(node, peer);
		if weight > 0.0 && best.is_none_or(|(_, top)| weight > top) {
			best = Some((*community, weight));
		}
	}

	best.map(|(community, _)| community)
}

fn largest(size: &HashMap<usize, usize>, smallest: usize) -> Option<usize> {
	size.iter()
		.filter(|(community, _)| **community != smallest)
		.max_by_key(|(community, count)| (**count, Reverse(**community)))
		.map(|(community, _)| *community)
}

#[cfg(test)]
mod tests {
	use super::{
		super::{fixture::seeded, similarity::similarity},
		*,
	};

	#[test]
	fn a_single_community_is_left_alone() {
		let mut label = vec![0, 0];
		let seed = seeded(&[&[1], &[2]]);
		small(&mut label, &similarity(&seed, 3), 10);

		assert_eq!(label, vec![0, 0]);
	}
}
