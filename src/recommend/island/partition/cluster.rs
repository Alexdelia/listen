use std::collections::HashMap;

use super::super::{real, seed::Seed};

pub(super) struct Similarity {
	value: Vec<f64>,
	seed: usize,
}

impl Similarity {
	pub(super) fn of(&self, a: usize, b: usize) -> f64 {
		self.value[a * self.seed + b]
	}
}

pub(super) fn similarity(seed: &[Seed], user: usize) -> Similarity {
	let word = user.div_ceil(u64::BITS as usize).max(1);
	let mut bit = vec![0u64; seed.len() * word];

	for (index, seed) in seed.iter().enumerate() {
		for listener in &seed.listener {
			let listener = *listener as usize;
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

pub(super) fn detect(
	similarity: &Similarity,
	threshold: f64,
	resolution: f64,
	min_member: usize,
) -> Vec<usize> {
	let node = similarity.seed;
	let mut edge = Vec::new();

	for a in 0..node {
		for b in (a + 1)..node {
			let weight = similarity.of(a, b);
			if weight >= threshold {
				edge.push((a, b, weight));
			}
		}
	}

	let mut label = louvain(node, &edge, resolution);
	absorb(&mut label, similarity, min_member);

	label
}

fn absorb(label: &mut [usize], similarity: &Similarity, min_member: usize) {
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
			let mut best: Option<(usize, f64)> = None;

			for (peer, community) in label.iter().enumerate() {
				if *community == smallest {
					continue;
				}

				let weight = similarity.of(node, peer);
				if best.is_none_or(|(_, top)| weight > top) {
					best = Some((*community, weight));
				}
			}

			if let Some((community, _)) = best {
				label[node] = community;
				moved = true;
			}
		}

		if !moved {
			return;
		}
	}
}

struct Graph {
	adj: Vec<Vec<(usize, f64)>>,
	self_loop: Vec<f64>,
	degree: Vec<f64>,
	total: f64,
}

impl Graph {
	fn of(node: usize, edge: &[(usize, usize, f64)]) -> Self {
		let mut graph = Self {
			adj: vec![Vec::new(); node],
			self_loop: vec![0.0; node],
			degree: vec![0.0; node],
			total: 0.0,
		};

		for (a, b, weight) in edge.iter().copied() {
			graph.total += 2.0 * weight;

			if a == b {
				graph.self_loop[a] += 2.0 * weight;
				graph.degree[a] += 2.0 * weight;
				continue;
			}

			graph.adj[a].push((b, weight));
			graph.adj[b].push((a, weight));
			graph.degree[a] += weight;
			graph.degree[b] += weight;
		}

		graph
	}

	fn shrink(&self, community: &[usize], count: usize) -> Self {
		let mut self_loop = vec![0.0; count];
		let mut degree = vec![0.0; count];
		let mut between: HashMap<(usize, usize), f64> = HashMap::new();

		for node in 0..self.adj.len() {
			let here = community[node];
			self_loop[here] += self.self_loop[node];
			degree[here] += self.degree[node];

			for (peer, weight) in self.adj[node].iter().copied() {
				let there = community[peer];

				if here == there {
					if node < peer {
						self_loop[here] += 2.0 * weight;
					}
				} else if here < there {
					*between.entry((here, there)).or_default() += weight;
				}
			}
		}

		let mut adj = vec![Vec::new(); count];
		for ((here, there), weight) in between {
			adj[here].push((there, weight));
			adj[there].push((here, weight));
		}

		Self {
			adj,
			self_loop,
			total: degree.iter().sum(),
			degree,
		}
	}
}

fn louvain(node: usize, edge: &[(usize, usize, f64)], resolution: f64) -> Vec<usize> {
	let mut graph = Graph::of(node, edge);
	let mut label: Vec<usize> = (0..node).collect();

	loop {
		let (community, count) = local_move(&graph, resolution);

		if count == graph.adj.len() {
			return label;
		}

		for label in &mut label {
			*label = community[*label];
		}

		graph = graph.shrink(&community, count);
	}
}

fn local_move(graph: &Graph, resolution: f64) -> (Vec<usize>, usize) {
	let node = graph.adj.len();
	let mut community: Vec<usize> = (0..node).collect();
	let mut weight = graph.degree.clone();

	if graph.total <= 0.0 {
		return dense(community);
	}

	loop {
		let mut moved = false;

		for here in 0..node {
			let leaving = community[here];
			weight[leaving] -= graph.degree[here];

			let mut reachable: HashMap<usize, f64> = HashMap::new();
			for (peer, edge) in graph.adj[here].iter().copied() {
				if peer != here {
					*reachable.entry(community[peer]).or_default() += edge;
				}
			}

			let gain = |target: usize, shared: f64| {
				shared - resolution * graph.degree[here] * weight[target] / graph.total
			};

			let mut best = (
				leaving,
				gain(leaving, reachable.get(&leaving).copied().unwrap_or(0.0)),
			);

			for (target, shared) in reachable {
				let value = gain(target, shared);
				if value > best.1 {
					best = (target, value);
				}
			}

			weight[best.0] += graph.degree[here];
			community[here] = best.0;

			if best.0 != leaving {
				moved = true;
			}
		}

		if !moved {
			return dense(community);
		}
	}
}

fn dense(community: Vec<usize>) -> (Vec<usize>, usize) {
	let mut seen: HashMap<usize, usize> = HashMap::new();

	let community = community
		.into_iter()
		.map(|label| {
			let next = seen.len();
			*seen.entry(label).or_insert(next)
		})
		.collect();

	(community, seen.len())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn edge(pair: &[(usize, usize)]) -> Vec<(usize, usize, f64)> {
		pair.iter().map(|(a, b)| (*a, *b, 1.0)).collect()
	}

	fn group(label: &[usize]) -> Vec<Vec<usize>> {
		let mut by: HashMap<usize, Vec<usize>> = HashMap::new();
		for (node, community) in label.iter().enumerate() {
			by.entry(*community).or_default().push(node);
		}

		let mut group: Vec<Vec<usize>> = by.into_values().collect();
		group.sort();
		group
	}

	#[test]
	fn two_disconnected_triangles_are_two_communities() {
		let label = louvain(
			6,
			&edge(&[(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)]),
			1.0,
		);

		assert_eq!(group(&label), vec![vec![0, 1, 2], vec![3, 4, 5]]);
	}

	#[test]
	fn two_triangles_joined_by_one_edge_stay_apart() {
		let label = louvain(
			6,
			&edge(&[(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5), (2, 3)]),
			1.0,
		);

		assert_eq!(group(&label), vec![vec![0, 1, 2], vec![3, 4, 5]]);
	}

	#[test]
	fn a_clique_is_one_community() {
		let label = louvain(
			4,
			&edge(&[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]),
			1.0,
		);

		assert_eq!(group(&label).len(), 1);
	}

	#[test]
	fn an_edgeless_graph_leaves_every_node_alone() {
		let label = louvain(3, &[], 1.0);

		assert_eq!(group(&label).len(), 3);
	}

	#[test]
	fn a_higher_resolution_splits_at_least_as_much() {
		let pair = edge(&[
			(0, 1),
			(1, 2),
			(0, 2),
			(2, 3),
			(3, 4),
			(4, 5),
			(3, 5),
			(5, 6),
			(6, 7),
			(7, 8),
			(6, 8),
		]);

		let coarse = group(&louvain(9, &pair, 0.5)).len();
		let fine = group(&louvain(9, &pair, 2.0)).len();

		assert!(fine >= coarse, "coarse {coarse} fine {fine}");
	}

	fn seeded(listener: &[&[u32]]) -> Vec<Seed> {
		listener
			.iter()
			.enumerate()
			.map(|(index, listener)| Seed {
				mbid: crate::declaration::Source::from_bytes([index as u8; 16]),
				q: 2,
				listener: listener.to_vec(),
				deliberate: listener.to_vec(),
			})
			.collect()
	}

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
	fn similarity_is_symmetric() {
		let seed = seeded(&[&[1, 2, 3], &[2, 3, 4], &[9]]);
		let similarity = similarity(&seed, 10);

		for a in 0..3 {
			for b in 0..3 {
				assert!((similarity.of(a, b) - similarity.of(b, a)).abs() < f64::EPSILON);
			}
		}
	}

	#[test]
	fn a_seed_with_no_listener_is_absorbed_by_its_nearest_neighbour() {
		let seed = seeded(&[&[1, 2, 3, 4], &[1, 2, 3, 4], &[1, 2], &[]]);
		let similarity = similarity(&seed, 5);
		let label = detect(&similarity, 0.15, 1.0, 3);

		assert_eq!(group(&label).len(), 1, "{label:?}");
	}

	#[test]
	fn absorb_leaves_a_single_community_alone() {
		let mut label = vec![0, 0];
		let seed = seeded(&[&[1], &[2]]);
		absorb(&mut label, &similarity(&seed, 3), 10);

		assert_eq!(label, vec![0, 0]);
	}
}
