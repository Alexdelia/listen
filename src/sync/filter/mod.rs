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
pub(super) struct GroupedEntry<T> {
	pub fs: T,

	pub q: HashMap<Q, T>,

	pub playlist: HashMap<String, T>,
}

#[derive(Default, Debug)]
pub(super) struct SyncEntry {
	pub add: Vec<Source>,
	pub remove: Vec<Source>,
}

const fn touched<K>(_: &K, sync: &mut SyncEntry) -> bool {
	!sync.add.is_empty() || !sync.remove.is_empty()
}

pub(super) fn sync(list: Vec<Entry>) -> hmerr::Result<GroupedEntry<SyncEntry>> {
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

	ret.fs.remove.extend(existing.fs);
	remove::grouped(existing.q, &mut ret.q);
	remove::grouped(existing.playlist, &mut ret.playlist);

	ret.q.retain(touched);
	ret.playlist.retain(touched);

	ret.fs.sort();
	sort::grouped(&mut ret.q);
	sort::grouped(&mut ret.playlist);

	Ok(ret)
}
