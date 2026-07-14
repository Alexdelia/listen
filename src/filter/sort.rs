use std::collections::HashMap;

use crate::entry::Q;

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

pub fn q(sync: &mut HashMap<Q, SyncEntry>) {
	for entry in sync.values_mut() {
		entry.sort();
	}
}

pub fn playlist(sync: &mut HashMap<String, SyncEntry>) {
	for entry in sync.values_mut() {
		entry.sort();
	}
}
