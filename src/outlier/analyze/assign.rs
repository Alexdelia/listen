use std::collections::{HashMap, HashSet};

use crate::declaration::{Entry, Source};

use super::super::{fetch::ListenCount, meta::Meta, song::Song};

pub(super) struct Assignment<'l> {
	pub per_entry: Vec<u32>,
	pub consumed: HashSet<&'l Source>,
}

pub(super) fn assign<'l>(list: &[Entry], listen: &'l ListenCount, meta: &Meta) -> Assignment<'l> {
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

		let matched = song
			.iter()
			.map(|s| s.as_ref().and_then(|s| s.matches(&listened)))
			.collect::<Vec<_>>();

		let Some(best) = matched.iter().flatten().copied().max() else {
			if let Some(i) = unique_title(&song, &listened) {
				per_entry[i] += l.count;
				consumed.insert(mbid);
			}
			continue;
		};

		consumed.insert(mbid);
		for (i, matched) in matched.iter().enumerate() {
			if *matched == Some(best) {
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
