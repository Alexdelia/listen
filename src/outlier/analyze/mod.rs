mod assign;
mod rate;
mod undeclared;

use std::collections::BTreeMap;

use crate::declaration::{Entry, Q, Source};

use super::{age::Age, fetch::ListenCount, gap::Covered, meta::Meta};

pub(super) use undeclared::Undeclared;

use assign::assign;
use rate::{median_per_q, nearest_q, rate};
use undeclared::undeclared;

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

pub(super) fn analyze(
	list: &[Entry],
	listen: &ListenCount,
	age: &Age,
	meta: &Meta,
	covered: &Covered,
) -> Analysis {
	let count = assign(list, listen, meta);
	let consumed = count.consumed;

	let observation = list
		.iter()
		.zip(count.per_entry)
		.map(|(entry, count)| {
			let days = covered.days(age.get(&entry.s).copied().unwrap_or(0));

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

	let median = median_per_q(
		considered
			.iter()
			.map(|(entry, _, _, rate)| (entry.q, *rate)),
	);

	let mut outlier = considered
		.into_iter()
		.filter_map(|(entry, listen, days, rate)| {
			let observed = nearest_q(&median, rate)?;

			(observed != entry.q).then_some(Record {
				mbid: entry.s,
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
			.then(b.rate.total_cmp(&a.rate))
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
#[cfg(test)]
mod tests {
	use super::*;

	use crate::outlier::{fetch::Listen, gap::Window};

	fn id(name: &str) -> Source {
		let mut bytes = [0; 16];
		bytes[..name.len()].copy_from_slice(name.as_bytes());
		Source::from_bytes(bytes)
	}

	fn entry(s: &str, q: Q) -> Entry {
		Entry {
			s: id(s),
			q,
			playlist: vec![],
		}
	}

	fn covered(ago: u64) -> Covered {
		Covered { ago, gap: vec![] }
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
			.map(|(s, _, c)| (id(s), play(*c, "", "")))
			.collect::<ListenCount>();
		let age = sample.iter().map(|(s, _, _)| (id(s), 100)).collect::<Age>();

		let analysis = analyze(&list, &count, &age, &Meta::new(), &covered(0));

		let by_mbid = analysis
			.outlier
			.iter()
			.map(|r| (r.mbid, r))
			.collect::<std::collections::HashMap<Source, &Record>>();

		assert_eq!(analysis.outlier.len(), 2);
		assert_eq!(by_mbid[&id("low-outlier")].observed, 4);
		assert_eq!(by_mbid[&id("high-outlier")].observed, 1);
	}

	#[test]
	fn missing_listen_counts_as_zero() {
		let list = vec![entry("declared", 4)];
		let age = Age::from([(id("declared"), 100)]);

		let analysis = analyze(&list, &ListenCount::new(), &age, &Meta::new(), &covered(0));

		assert_eq!(analysis.outlier.len(), 0);
		assert_eq!(analysis.median.get(&4), Some(&0.0));
	}

	#[test]
	fn young_entry_is_excluded() {
		let list = vec![entry("fresh", 4)];
		let count = ListenCount::from([(id("fresh"), play(1, "", ""))]);
		let age = Age::from([(id("fresh"), MIN_DAY - 1)]);

		let analysis = analyze(&list, &count, &age, &Meta::new(), &covered(0));

		assert!(analysis.median.is_empty());
		assert!(analysis.outlier.is_empty());
	}

	#[test]
	fn listen_matched_across_mbid() {
		let list = vec![entry("a", 1), entry("b", 4), entry("hole", 4)];
		let count = ListenCount::from([
			(id("a"), play(1, "A", "X")),
			(id("b"), play(100, "B", "Y")),
			(id("scrobbled"), play(100, "Hole Song", "Z")),
		]);
		let age = sample_age(&["a", "b", "hole"]);
		let meta = Meta::from([(id("hole"), ("Hole Song".to_string(), "Z".to_string()))]);

		let analysis = analyze(&list, &count, &age, &meta, &covered(0));

		assert!(
			analysis
				.undeclared
				.iter()
				.all(|u| u.mbid != id("scrobbled"))
		);
		assert!(analysis.outlier.iter().all(|o| o.mbid != id("hole")));
	}

	#[test]
	fn version_listen_stays_on_its_own_version() {
		let list = vec![entry("original", 1), entry("remix", 4)];
		let count = ListenCount::from([
			(id("play-original"), play(10, "Collide", "Hellberg")),
			(
				id("play-remix"),
				play(100, "Collide (Astronaut & Barely Alive remix)", "Hellberg"),
			),
		]);
		let age = sample_age(&["original", "remix"]);
		let meta = Meta::from([
			(
				id("original"),
				("Collide".to_string(), "Hellberg".to_string()),
			),
			(
				id("remix"),
				(
					"Collide (Astronaut & Barely Alive remix)".to_string(),
					"Hellberg".to_string(),
				),
			),
		]);

		let analysis = analyze(&list, &count, &age, &meta, &covered(0));

		assert_eq!(analysis.median.get(&1), Some(&0.1));
		assert_eq!(analysis.median.get(&4), Some(&1.0));
	}

	#[test]
	fn wrong_artist_still_matches_a_unique_title() {
		let list = vec![entry("declared", 3)];
		let count = ListenCount::from([(
			id("mismatched"),
			play(30, "Gnossienne no. 1", "Pascal Rogé"),
		)]);
		let age = sample_age(&["declared"]);
		let meta = Meta::from([(
			id("declared"),
			("Gnossienne no. 1".to_string(), "Otto Tolonen".to_string()),
		)]);

		let analysis = analyze(&list, &count, &age, &meta, &covered(0));

		assert!(analysis.undeclared.is_empty());
		assert_eq!(analysis.matched, 1);
	}

	#[test]
	fn ambiguous_title_stays_undeclared() {
		let list = vec![entry("cover-a", 2), entry("cover-b", 2)];
		let count = ListenCount::from([(id("other-cover"), play(9, "Bad Apple!!", "Mini Miku"))]);
		let age = sample_age(&["cover-a", "cover-b"]);
		let meta = Meta::from([
			(
				id("cover-a"),
				(
					"Bad Apple!!".to_string(),
					"RichaadEB & Cristina Vee".to_string(),
				),
			),
			(
				id("cover-b"),
				("Bad Apple".to_string(), "Cloudjumper & UN3H".to_string()),
			),
		]);

		let analysis = analyze(&list, &count, &age, &meta, &covered(0));

		assert_eq!(analysis.undeclared.len(), 1);
		assert_eq!(analysis.matched, 0);
	}

	#[test]
	fn an_entry_declared_after_what_the_count_covers_is_not_judged() {
		let list = vec![entry("declared-after", 4), entry("declared-before", 1)];
		let count = ListenCount::from([(id("declared-before"), play(50, "", ""))]);
		let age = Age::from([(id("declared-after"), 30), (id("declared-before"), 300)]);

		let analysis = analyze(&list, &count, &age, &Meta::new(), &covered(42));

		assert!(!analysis.median.contains_key(&4));
		assert!(
			analysis
				.outlier
				.iter()
				.all(|o| o.mbid != id("declared-after"))
		);
	}

	#[test]
	fn a_rate_is_over_the_days_the_count_covers_not_over_the_whole_age() {
		let list = vec![entry("declared", 4)];
		let count = ListenCount::from([(id("declared"), play(60, "", ""))]);
		let age = Age::from([(id("declared"), 100)]);

		let covering = analyze(&list, &count, &age, &Meta::new(), &covered(40));
		let whole = analyze(&list, &count, &age, &Meta::new(), &covered(0));

		assert_eq!(covering.median.get(&4), Some(&1.0));
		assert_eq!(whole.median.get(&4), Some(&0.6));
	}

	#[test]
	fn a_rate_is_over_the_days_the_count_saw_not_over_the_ones_no_dump_covered() {
		let list = vec![entry("declared", 4)];
		let count = ListenCount::from([(id("declared"), play(60, "", ""))]);
		let age = Age::from([(id("declared"), 100)]);

		let holed = analyze(
			&list,
			&count,
			&age,
			&Meta::new(),
			&Covered {
				ago: 0,
				gap: vec![Window { from: 50, to: 20 }],
			},
		);

		let whole = analyze(&list, &count, &age, &Meta::new(), &covered(0));

		assert_eq!(holed.median.get(&4), Some(&(60.0 / 70.0)));
		assert_eq!(whole.median.get(&4), Some(&0.6));
	}

	fn sample_age(mbid: &[&str]) -> Age {
		mbid.iter().map(|s| (id(s), 100)).collect()
	}
}
