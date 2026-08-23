use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
};

use crate::declaration::Source;

use super::SyncEntry;

pub(super) fn fs(existing: HashSet<Source>, remove: &mut Vec<Source>) {
	remove.extend(existing);
}

pub(super) fn grouped<K>(existing: HashMap<K, HashSet<Source>>, remove: &mut HashMap<K, SyncEntry>)
where
	K: Eq + Hash,
{
	for (key, set) in existing {
		remove.entry(key).or_default().remove.extend(set);
	}
}
