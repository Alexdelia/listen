mod existing;

use std::{collections::HashMap, fs, path::Path};

use hmerr::ioe;

use crate::{
	entry::{Entry, Source, Q},
	playlist,
};

#[derive(Default)]
pub struct GroupedEntry<T> {
	pub fs: T,

	pub q: HashMap<Q, T>,

	pub playlist: HashMap<String, T>,
}

#[derive(Default)]
pub struct SyncEntry {
	pub add: Vec<Source>,
	pub remove: Vec<Source>,
}

pub fn sync(list: Vec<Entry>) -> hmerr::Result<GroupedEntry<SyncEntry>> {
	let ret = GroupedEntry::default();

	fs::create_dir_all(Entry::OUTPUT_DIR).map_err(|e| ioe!(Entry::OUTPUT_DIR, e))?;
	fs::create_dir_all(playlist::OUTPUT_DIR).map_err(|e| ioe!(playlist::OUTPUT_DIR, e))?;

	let existing = existing::get()?;

	for entry in list {
		let path = Path::new(Entry::OUTPUT_DIR)
			.join(entry.s)
			.with_extension(Entry::EXT);

		dbg!(&path);
		dbg!(path.exists());
	}

	Ok(ret)
}
