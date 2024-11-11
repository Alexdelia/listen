use std::collections::{HashMap, HashSet};

use crate::entry::{Source, Q};

use super::SyncEntry;

pub fn fs(existing: &HashSet<Source>, remove: &mut Vec<Source>) {
	for source in existing {
		remove.push(source.clone());
	}
}

pub fn q(existing: &HashMap<Q, HashSet<Source>>, remove: &mut HashMap<Q, SyncEntry>) {
	for (q, set) in existing {
		if remove.get(q).is_none() {
			remove.insert(*q, SyncEntry::default());
		}

		for source in set {
			remove.get_mut(q).unwrap().remove.push(source.clone());
		}
	}
}

pub fn playlist(
	existing: &HashMap<String, HashSet<Source>>,
	remove: &mut HashMap<String, SyncEntry>,
) {
	for (playlist, set) in existing {
		if remove.get(playlist).is_none() {
			remove.insert(playlist.clone(), SyncEntry::default());
		}

		for source in set {
			remove
				.get_mut(playlist)
				.unwrap()
				.remove
				.push(source.clone());
		}
	}
}
