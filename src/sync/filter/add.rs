use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
};

use crate::declaration::{Entry, Q, Source};

use super::SyncEntry;

pub fn fs(existing: &mut HashSet<Source>, add: &mut Vec<Source>, entry: &Entry) {
	if existing.contains(&entry.s) {
		existing.remove(&entry.s);
	} else {
		add.push(entry.s);
	}
}

pub fn q(
	existing: &mut HashMap<Q, HashSet<Source>>,
	add: &mut HashMap<Q, SyncEntry>,
	entry: &Entry,
) {
	for q in 0..=entry.q {
		grouped(existing, add, &q, entry.s);
	}
}

pub fn playlist(
	existing: &mut HashMap<String, HashSet<Source>>,
	add: &mut HashMap<String, SyncEntry>,
	entry: &Entry,
) {
	for playlist in &entry.playlist {
		grouped(existing, add, playlist, entry.s);
	}
}

fn grouped<K>(
	existing: &mut HashMap<K, HashSet<Source>>,
	add: &mut HashMap<K, SyncEntry>,
	key: &K,
	source: Source,
) where
	K: Clone + Eq + Hash,
{
	if let Some(set) = existing.get_mut(key)
		&& set.contains(&source)
	{
		set.remove(&source);
		return;
	}

	add.entry(key.clone()).or_default().add.push(source);
}
