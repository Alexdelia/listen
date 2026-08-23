use std::collections::HashMap;

use crate::declaration::Source;

use super::super::super::seed::{Listener, Seed};

pub(super) fn seeded(listener: &[&[u32]]) -> Vec<Seed> {
	listener
		.iter()
		.enumerate()
		.map(|(index, listener)| Seed {
			mbid: Source::from_bytes([u8::try_from(index).unwrap_or_default(); 16]),
			q: 2,
			listener: listener
				.iter()
				.map(|user| Listener {
					user: *user,
					weight: 1.0,
				})
				.collect(),
		})
		.collect()
}

pub(super) fn edge(pair: &[(usize, usize)]) -> Vec<(usize, usize, f64)> {
	pair.iter().map(|(a, b)| (*a, *b, 1.0)).collect()
}

pub(super) fn group(label: &[usize]) -> Vec<Vec<usize>> {
	let mut by: HashMap<usize, Vec<usize>> = HashMap::new();
	for (node, community) in label.iter().enumerate() {
		by.entry(*community).or_default().push(node);
	}

	let mut group: Vec<Vec<usize>> = by.into_values().collect();
	group.sort();
	group
}
