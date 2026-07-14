use std::{
	cmp::{Ordering, Reverse},
	collections::{BTreeMap, HashSet},
};

use crate::entry::{Entry, Q, Source};

use super::{age::Age, fetch::ListenCount};

pub(super) struct Analysis {
	pub median: BTreeMap<Q, f64>,
	pub outlier: Vec<Record>,
	pub undeclared: Vec<Undeclared>,
	pub matched: usize,
	pub declared: usize,
}

pub(super) struct Record {
	pub mbid: Source,
	pub declared: Q,
	pub observed: Q,
	pub listen: u32,
	pub days: u64,
	pub rate: f64,
}

pub(super) struct Undeclared {
	pub mbid: Source,
	pub listen: u32,
	pub track: String,
	pub artist: String,
}

pub(super) fn analyze(list: &[Entry], listen: &ListenCount, age: &Age) -> Analysis {
	let observation = list
		.iter()
		.map(|entry| {
			let count = listen.get(&entry.s).map_or(0, |l| l.count);
			let days = age.get(&entry.s).copied().unwrap_or(0);

			(entry, count, days, rate(count, days))
		})
		.collect::<Vec<_>>();

	let matched = observation
		.iter()
		.filter(|(_, count, ..)| *count > 0)
		.count();

	let undeclared = undeclared(list, listen);

	let median = median_per_q(&observation);

	let mut outlier = observation
		.into_iter()
		.filter_map(|(entry, listen, days, rate)| {
			let observed = nearest_q(&median, rate)?;

			(observed != entry.q).then(|| Record {
				mbid: entry.s.clone(),
				declared: entry.q,
				observed,
				listen,
				days,
				rate,
			})
		})
		.collect::<Vec<_>>();

	outlier.sort_by(|a, b| {
		b.declared
			.abs_diff(b.observed)
			.cmp(&a.declared.abs_diff(a.observed))
			.then(cmp_rate(b.rate, a.rate))
	});

	Analysis {
		median,
		outlier,
		undeclared,
		matched,
		declared: list.len(),
	}
}

fn undeclared(list: &[Entry], listen: &ListenCount) -> Vec<Undeclared> {
	let declared = list.iter().map(|entry| &entry.s).collect::<HashSet<_>>();

	let mut undeclared = listen
		.iter()
		.filter(|(mbid, _)| !declared.contains(mbid))
		.map(|(mbid, l)| Undeclared {
			mbid: mbid.clone(),
			listen: l.count,
			track: l.track.clone(),
			artist: l.artist.clone(),
		})
		.collect::<Vec<_>>();

	undeclared.sort_by_key(|undeclared| Reverse(undeclared.listen));

	undeclared
}

#[allow(
	clippy::cast_precision_loss,
	reason = "listen count and day span stay far below 2^53, so the conversion is exact"
)]
fn rate(listen: u32, days: u64) -> f64 {
	f64::from(listen) / days.max(1) as f64
}

fn median_per_q(observation: &[(&Entry, u32, u64, f64)]) -> BTreeMap<Q, f64> {
	let mut per_q: BTreeMap<Q, Vec<f64>> = BTreeMap::new();

	for (entry, _, _, rate) in observation {
		per_q.entry(entry.q).or_default().push(*rate);
	}

	per_q
		.into_iter()
		.map(|(q, mut rate)| (q, median(&mut rate)))
		.collect()
}

fn median(rate: &mut [f64]) -> f64 {
	rate.sort_by(|a, b| cmp_rate(*a, *b));

	match rate.len() {
		0 => 0.0,
		n if n % 2 == 1 => rate[n / 2],
		n => f64::midpoint(rate[n / 2 - 1], rate[n / 2]),
	}
}

fn nearest_q(median: &BTreeMap<Q, f64>, rate: f64) -> Option<Q> {
	median
		.iter()
		.min_by(|(_, a), (_, b)| cmp_rate((rate - **a).abs(), (rate - **b).abs()))
		.map(|(q, _)| *q)
}

fn cmp_rate(a: f64, b: f64) -> Ordering {
	a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests {
	use super::*;

	use crate::outlier::fetch::Listen;

	fn entry(s: &str, q: Q) -> Entry {
		Entry {
			s: s.to_string(),
			q,
			playlist: vec![],
		}
	}

	fn listen(count: u32) -> Listen {
		Listen {
			count,
			track: String::new(),
			artist: String::new(),
		}
	}

	#[test]
	fn overrated_and_underrated() {
		let sample = [
			("low-a", 1, 1),
			("low-b", 1, 2),
			("low-outlier", 1, 100),
			("high-a", 4, 100),
			("high-b", 4, 99),
			("high-outlier", 4, 1),
		];

		let list = sample
			.iter()
			.map(|(s, q, _)| entry(s, *q))
			.collect::<Vec<_>>();
		let count = sample
			.iter()
			.map(|(s, _, c)| ((*s).to_string(), listen(*c)))
			.collect::<ListenCount>();
		let age = sample
			.iter()
			.map(|(s, _, _)| ((*s).to_string(), 1))
			.collect::<Age>();

		let analysis = analyze(&list, &count, &age);

		let by_mbid = analysis
			.outlier
			.iter()
			.map(|r| (r.mbid.as_str(), r))
			.collect::<std::collections::HashMap<&str, &Record>>();

		assert_eq!(analysis.outlier.len(), 2);
		assert_eq!(by_mbid["low-outlier"].observed, 4);
		assert_eq!(by_mbid["high-outlier"].observed, 1);
	}

	#[test]
	fn missing_listen_counts_as_zero() {
		let list = vec![entry("declared", 4)];
		let analysis = analyze(&list, &ListenCount::new(), &Age::new());

		assert_eq!(analysis.outlier.len(), 0);
		assert_eq!(analysis.median.get(&4), Some(&0.0));
	}
}
