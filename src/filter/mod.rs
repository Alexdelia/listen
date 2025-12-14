mod add;
mod existing;
mod remove;
mod sort;

use std::{collections::HashMap, fs};

use hmerr::ioe;

use crate::{
	entry::{Entry, Q, Source},
	playlist,
};

#[derive(Default, Debug)]
pub struct GroupedEntry<T> {
	pub fs: T,

	pub q: HashMap<Q, T>,

	pub playlist: HashMap<String, T>,
}

#[derive(Default, Debug)]
pub struct SyncEntry {
	pub add: Vec<Source>,
	pub remove: Vec<Source>,
}

pub fn sync(list: Vec<Entry>) -> hmerr::Result<GroupedEntry<SyncEntry>> {
	let mut ret = GroupedEntry::<SyncEntry>::default();

	fs::create_dir_all(Entry::OUTPUT_DIR).map_err(|e| ioe!(Entry::OUTPUT_DIR, e))?;
	fs::create_dir_all(playlist::OUTPUT_DIR).map_err(|e| ioe!(playlist::OUTPUT_DIR, e))?;

	let mut existing = existing::get()?;

	for entry in list {
		add::fs(&mut existing.fs, &mut ret.fs.add, &entry);
		add::q(&mut existing.q, &mut ret.q, &entry);
		add::playlist(&mut existing.playlist, &mut ret.playlist, &entry);
	}

	remove::fs(&existing.fs, &mut ret.fs.remove);
	remove::q(&existing.q, &mut ret.q);
	remove::playlist(&existing.playlist, &mut ret.playlist);

	ret.q
		.retain(|_, v| !v.add.is_empty() || !v.remove.is_empty());
	ret.playlist
		.retain(|_, v| !v.add.is_empty() || !v.remove.is_empty());

	sort::fs(&mut ret.fs);
	sort::q(&mut ret.q);
	sort::playlist(&mut ret.playlist);

	Ok(ret)
}
