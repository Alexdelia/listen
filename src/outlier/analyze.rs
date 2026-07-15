use std::{
	cmp::{Ordering, Reverse},
	collections::{BTreeMap, HashMap, HashSet},
};

use crate::declaration::{Entry, Q, Source};

use super::{age::Age, fetch::ListenCount, meta::Meta, song::Song};

const MIN_DAY: u64 = 21;

pub(super) struct Analysis {
	pub median: BTreeMap<Q, f64>,
	pub declared_per_q: BTreeMap<Q, usize>,
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

pub(super) fn analyze(list: &[Entry], listen: &ListenCount, age: &Age, meta: &Meta) -> Analysis {
	let count = assign(list, listen, meta);
	let consumed = count.consumed;

	let observation = list
		.iter()
		.zip(count.per_entry)
		.map(|(entry, count)| {
			let days = age.get(&entry.s).copied().unwrap_or(0);

			(entry, count, days, rate(count, days))
		})
		.collect::<Vec<_>>();

	let matched = observation
		.iter()
		.filter(|(_, count, ..)| *count > 0)
		.count();

	let considered = observation
		.iter()
		.copied()
		.filter(|(_, _, days, _)| *days >= MIN_DAY)
		.collect::<Vec<_>>();

	let median = median_per_q(&considered);

	let mut outlier = considered
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

	let undeclared = undeclared(listen, &consumed);

	Analysis {
		median,
		declared_per_q: declared_per_q(list),
		outlier,
		undeclared,
		matched,
		declared: list.len(),
	}
}

fn declared_per_q(list: &[Entry]) -> BTreeMap<Q, usize> {
	let mut per_q = BTreeMap::new();

	for entry in list {
		*per_q.entry(entry.q).or_insert(0) += 1;
	}

	per_q
}

struct Assignment<'l> {
	per_entry: Vec<u32>,
	consumed: HashSet<&'l Source>,
}

fn assign<'l>(list: &[Entry], listen: &'l ListenCount, meta: &Meta) -> Assignment<'l> {
	let song = list
		.iter()
		.map(|entry| {
			meta.get(&entry.s)
				.map(|(title, artist)| Song::new(title, artist))
		})
		.collect::<Vec<_>>();

	let index = list
		.iter()
		.enumerate()
		.map(|(i, entry)| (&entry.s, i))
		.collect::<HashMap<_, _>>();

	let mut per_entry = vec![0u32; list.len()];
	let mut consumed = HashSet::new();

	for (mbid, l) in listen {
		if let Some(&i) = index.get(mbid) {
			per_entry[i] += l.count;
			consumed.insert(mbid);
			continue;
		}

		let listened = Song::new(&l.track, &l.artist);

		let Some(best) = song
			.iter()
			.flatten()
			.filter_map(|s| s.matches(&listened))
			.max()
		else {
			if let Some(i) = unique_title(&song, &listened) {
				per_entry[i] += l.count;
				consumed.insert(mbid);
			}
			continue;
		};

		consumed.insert(mbid);
		for (i, s) in song.iter().enumerate() {
			if s.as_ref().and_then(|s| s.matches(&listened)) == Some(best) {
				per_entry[i] += l.count;
			}
		}
	}

	Assignment {
		per_entry,
		consumed,
	}
}

fn unique_title(song: &[Option<Song>], listened: &Song) -> Option<usize> {
	unique(song, |s| s.same_title(listened))
		.or_else(|| unique(song, |s| s.same_stripped_title(listened)))
}

fn unique(song: &[Option<Song>], matches: impl Fn(&Song) -> bool) -> Option<usize> {
	let mut candidate = song
		.iter()
		.enumerate()
		.filter(|(_, s)| s.as_ref().is_some_and(&matches))
		.map(|(i, _)| i);

	let first = candidate.next()?;
	candidate.next().is_none().then_some(first)
}

fn undeclared(listen: &ListenCount, consumed: &HashSet<&Source>) -> Vec<Undeclared> {
	let mut undeclared = listen
		.iter()
		.filter(|(mbid, _)| !consumed.contains(mbid))
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

	fn play(count: u32, track: &str, artist: &str) -> Listen {
		Listen {
			count,
			track: track.to_string(),
			artist: artist.to_string(),
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
			.map(|(s, _, c)| ((*s).to_string(), play(*c, "", "")))
			.collect::<ListenCount>();
		let age = sample
			.iter()
			.map(|(s, _, _)| ((*s).to_string(), 100))
			.collect::<Age>();

		let analysis = analyze(&list, &count, &age, &Meta::new());

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
		let age = Age::from([("declared".to_string(), 100)]);

		let analysis = analyze(&list, &ListenCount::new(), &age, &Meta::new());

		assert_eq!(analysis.outlier.len(), 0);
		assert_eq!(analysis.median.get(&4), Some(&0.0));
	}

	#[test]
	fn young_entry_is_excluded() {
		let list = vec![entry("fresh", 4)];
		let count = ListenCount::from([("fresh".to_string(), play(1, "", ""))]);
		let age = Age::from([("fresh".to_string(), MIN_DAY - 1)]);

		let analysis = analyze(&list, &count, &age, &Meta::new());

		assert!(analysis.median.is_empty());
		assert!(analysis.outlier.is_empty());
	}

	#[test]
	fn listen_matched_across_mbid() {
		let list = vec![entry("a", 1), entry("b", 4), entry("hole", 4)];
		let count = ListenCount::from([
			("a".to_string(), play(1, "A", "X")),
			("b".to_string(), play(100, "B", "Y")),
			("scrobbled".to_string(), play(100, "Hole Song", "Z")),
		]);
		let age = sample_age(&["a", "b", "hole"]);
		let meta = Meta::from([(
			"hole".to_string(),
			("Hole Song".to_string(), "Z".to_string()),
		)]);

		let analysis = analyze(&list, &count, &age, &meta);

		assert!(analysis.undeclared.iter().all(|u| u.mbid != "scrobbled"));
		assert!(analysis.outlier.iter().all(|o| o.mbid != "hole"));
	}

	#[test]
	fn version_listen_stays_on_its_own_version() {
		let list = vec![entry("original", 1), entry("remix", 4)];
		let count = ListenCount::from([
			("play-original".to_string(), play(10, "Collide", "Hellberg")),
			(
				"play-remix".to_string(),
				play(100, "Collide (Astronaut & Barely Alive remix)", "Hellberg"),
			),
		]);
		let age = sample_age(&["original", "remix"]);
		let meta = Meta::from([
			(
				"original".to_string(),
				("Collide".to_string(), "Hellberg".to_string()),
			),
			(
				"remix".to_string(),
				(
					"Collide (Astronaut & Barely Alive remix)".to_string(),
					"Hellberg".to_string(),
				),
			),
		]);

		let analysis = analyze(&list, &count, &age, &meta);

		assert_eq!(analysis.median.get(&1), Some(&0.1));
		assert_eq!(analysis.median.get(&4), Some(&1.0));
	}

	#[test]
	fn wrong_artist_still_matches_a_unique_title() {
		let list = vec![entry("declared", 3)];
		let count = ListenCount::from([(
			"mismatched".to_string(),
			play(30, "Gnossienne no. 1", "Pascal Rogé"),
		)]);
		let age = sample_age(&["declared"]);
		let meta = Meta::from([(
			"declared".to_string(),
			("Gnossienne no. 1".to_string(), "Otto Tolonen".to_string()),
		)]);

		let analysis = analyze(&list, &count, &age, &meta);

		assert!(analysis.undeclared.is_empty());
		assert_eq!(analysis.matched, 1);
	}

	#[test]
	fn ambiguous_title_stays_undeclared() {
		let list = vec![entry("cover-a", 2), entry("cover-b", 2)];
		let count = ListenCount::from([(
			"other-cover".to_string(),
			play(9, "Bad Apple!!", "Mini Miku"),
		)]);
		let age = sample_age(&["cover-a", "cover-b"]);
		let meta = Meta::from([
			(
				"cover-a".to_string(),
				(
					"Bad Apple!!".to_string(),
					"RichaadEB & Cristina Vee".to_string(),
				),
			),
			(
				"cover-b".to_string(),
				("Bad Apple".to_string(), "Cloudjumper & UN3H".to_string()),
			),
		]);

		let analysis = analyze(&list, &count, &age, &meta);

		assert_eq!(analysis.undeclared.len(), 1);
		assert_eq!(analysis.matched, 0);
	}

	fn sample_age(mbid: &[&str]) -> Age {
		mbid.iter().map(|s| ((*s).to_string(), 100)).collect()
	}
}
