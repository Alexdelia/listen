use std::collections::HashMap;

use super::SyncEntry;

impl SyncEntry {
	pub fn sort(&mut self) {
		self.add.sort();
		self.remove.sort();
	}
}

pub fn fs(sync: &mut SyncEntry) {
	sync.sort();
}

pub fn grouped<K>(sync: &mut HashMap<K, SyncEntry>) {
	for entry in sync.values_mut() {
		entry.sort();
	}
}
