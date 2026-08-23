use std::collections::HashMap;

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
			graph.total = 2.0f64.mul_add(weight, graph.total);

			if a == b {
				graph.self_loop[a] = 2.0f64.mul_add(weight, graph.self_loop[a]);
				graph.degree[a] = 2.0f64.mul_add(weight, graph.degree[a]);
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
						self_loop[here] = 2.0f64.mul_add(weight, self_loop[here]);
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

pub(super) fn of(node: usize, edge: &[(usize, usize, f64)], resolution: f64) -> Vec<usize> {
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
	use super::{
		super::fixture::{edge, group},
		*,
	};

	#[test]
	fn two_disconnected_triangles_are_two_communities() {
		let label = of(
			6,
			&edge(&[(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)]),
			1.0,
		);

		assert_eq!(group(&label), vec![vec![0, 1, 2], vec![3, 4, 5]]);
	}

	#[test]
	fn two_triangles_joined_by_one_edge_stay_apart() {
		let label = of(
			6,
			&edge(&[(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5), (2, 3)]),
			1.0,
		);

		assert_eq!(group(&label), vec![vec![0, 1, 2], vec![3, 4, 5]]);
	}

	#[test]
	fn a_clique_is_one_community() {
		let label = of(
			4,
			&edge(&[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]),
			1.0,
		);

		assert_eq!(group(&label).len(), 1);
	}

	#[test]
	fn an_edgeless_graph_leaves_every_node_alone() {
		let label = of(3, &[], 1.0);

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

		let coarse = group(&of(9, &pair, 0.5)).len();
		let fine = group(&of(9, &pair, 2.0)).len();

		assert!(fine >= coarse, "coarse {coarse} fine {fine}");
	}
}
