mod add;
mod remove;
mod sort;

use std::{collections::HashMap, fs};

use hmerr::ioe;

use crate::{
	declaration::{Entry, Q, Source},
	library,
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

	fs::create_dir_all(library::recording::DIR).map_err(|e| ioe!(library::recording::DIR, e))?;
	fs::create_dir_all(library::playlist::DIR).map_err(|e| ioe!(library::playlist::DIR, e))?;

	let m3u = library::playlist::existing()?;
	let mut existing = GroupedEntry {
		fs: library::recording::existing()?,
		q: m3u.q,
		playlist: m3u.playlist,
	};

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
