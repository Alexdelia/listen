use std::collections::{HashMap, HashSet};

use crate::entry::{Entry, Q, Source};

use super::SyncEntry;

pub fn fs(existing: &mut HashSet<Source>, add: &mut Vec<Source>, entry: &Entry) {
	if existing.contains(&entry.s) {
		existing.remove(&entry.s);
	} else {
		add.push(entry.s.clone());
	}
}

pub fn q(
	existing: &mut HashMap<Q, HashSet<Source>>,
	add: &mut HashMap<Q, SyncEntry>,
	entry: &Entry,
) {
	for q in 0..=entry.q {
		if let Some(q) = existing.get_mut(&q)
			&& q.contains(&entry.s)
		{
			q.remove(&entry.s);
			continue;
		}

		add.entry(q).or_default().add.push(entry.s.clone());
	}
}

pub fn playlist(
	existing: &mut HashMap<String, HashSet<Source>>,
	add: &mut HashMap<String, SyncEntry>,
	entry: &Entry,
) {
	for playlist in &entry.playlist {
		if let Some(set) = existing.get_mut(playlist)
			&& set.contains(&entry.s)
		{
			set.remove(&entry.s);
			continue;
		}

		add.entry(playlist.clone())
			.or_default()
			.add
			.push(entry.s.clone());
	}
}
