use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
};

use crate::declaration::Source;

use super::SyncEntry;

pub fn fs(existing: &HashSet<Source>, remove: &mut Vec<Source>) {
	remove.extend(existing.iter().copied());
}

pub fn grouped<K>(existing: &HashMap<K, HashSet<Source>>, remove: &mut HashMap<K, SyncEntry>)
where
	K: Clone + Eq + Hash,
{
	for (key, set) in existing {
		remove
			.entry(key.clone())
			.or_default()
			.remove
			.extend(set.iter().copied());
	}
}
