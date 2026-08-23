use std::{cmp::Ordering, collections::BTreeMap};

use crate::declaration::Q;

#[allow(
	clippy::cast_precision_loss,
	reason = "listen count and day span stay far below 2^53, so the conversion is exact"
)]
pub(super) fn rate(listen: u32, days: u64) -> f64 {
	f64::from(listen) / days.max(1) as f64
}

pub(super) fn median_per_q(observation: impl Iterator<Item = (Q, f64)>) -> BTreeMap<Q, f64> {
	let mut per_q: BTreeMap<Q, Vec<f64>> = BTreeMap::new();

	for (q, rate) in observation {
		per_q.entry(q).or_default().push(rate);
	}

	per_q
		.into_iter()
		.map(|(q, mut rate)| (q, median(&mut rate)))
		.collect()
}

pub(super) fn nearest_q(median: &BTreeMap<Q, f64>, rate: f64) -> Option<Q> {
	median
		.iter()
		.min_by(|(_, a), (_, b)| cmp_rate((rate - **a).abs(), (rate - **b).abs()))
		.map(|(q, _)| *q)
}

pub(super) fn cmp_rate(a: f64, b: f64) -> Ordering {
	a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn median(rate: &mut [f64]) -> f64 {
	rate.sort_by(|a, b| cmp_rate(*a, *b));

	match rate.len() {
		0 => 0.0,
		n if n % 2 == 1 => rate[n / 2],
		n => f64::midpoint(rate[n / 2 - 1], rate[n / 2]),
	}
}
