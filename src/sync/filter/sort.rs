use std::collections::HashMap;

use super::SyncEntry;

impl SyncEntry {
	pub(crate) fn sort(&mut self) {
		self.add.sort();
		self.remove.sort();
	}
}

pub(super) fn fs(sync: &mut SyncEntry) {
	sync.sort();
}

pub(super) fn grouped<K>(sync: &mut HashMap<K, SyncEntry>) {
	for entry in sync.values_mut() {
		entry.sort();
	}
}
