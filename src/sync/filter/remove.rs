use std::collections::{HashMap, HashSet};

use crate::declaration::{Q, Source};

use super::SyncEntry;

pub fn fs(existing: &HashSet<Source>, remove: &mut Vec<Source>) {
	for source in existing {
		remove.push(*source);
	}
}

pub fn q(existing: &HashMap<Q, HashSet<Source>>, remove: &mut HashMap<Q, SyncEntry>) {
	for (q, set) in existing {
		let entry = remove.entry(*q).or_default();
		for source in set {
			entry.remove.push(*source);
		}
	}
}

pub fn playlist(
	existing: &HashMap<String, HashSet<Source>>,
	remove: &mut HashMap<String, SyncEntry>,
) {
	for (playlist, set) in existing {
		let entry = remove.entry(playlist.clone()).or_default();
		for source in set {
			entry.remove.push(*source);
		}
	}
}
