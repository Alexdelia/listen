use std::collections::HashMap;

use super::SyncEntry;

impl SyncEntry {
	pub(super) fn sort(&mut self) {
		self.add.sort();
		self.remove.sort();
	}
}

pub(super) fn grouped<K>(sync: &mut HashMap<K, SyncEntry>) {
	for entry in sync.values_mut() {
		entry.sort();
	}
}
