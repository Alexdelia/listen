mod existing;

use std::{
	collections::{HashMap, HashSet},
	fs,
	path::Path,
};

use hmerr::ioe;

use crate::{
	entry::{Entry, Source, Q},
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
	dbg!(&existing);

	for entry in list {
		add_fs(&mut existing.fs, &mut ret.fs.add, &entry);
		add_q(&mut existing.q, &mut ret.q, &entry);
		add_playlist(&mut existing.playlist, &mut ret.playlist, &entry);
	}

	remove_fs(&existing.fs, &mut ret.fs.remove);
	remove_q(&existing.q, &mut ret.q);
	remove_playlist(&existing.playlist, &mut ret.playlist);

	dbg!(&ret);

	Ok(ret)
}

fn add_fs(existing: &mut HashSet<Source>, add: &mut Vec<Source>, entry: &Entry) {
	if existing.contains(&entry.s) {
		existing.remove(&entry.s);
	} else {
		add.push(entry.s.clone());
	}
}

fn add_q(
	existing: &mut HashMap<Q, HashSet<Source>>,
	add: &mut HashMap<Q, SyncEntry>,
	entry: &Entry,
) {
	for q in 0..=entry.q {
		if let Some(q) = existing.get_mut(&q) {
			if q.contains(&entry.s) {
				q.remove(&entry.s);
			}
		} else {
			if add.get(&q).is_none() {
				add.insert(q, SyncEntry::default());
			}

			add.get_mut(&q).unwrap().add.push(entry.s.clone());
		}
	}
}

fn add_playlist(
	existing: &mut HashMap<String, HashSet<Source>>,
	add: &mut HashMap<String, SyncEntry>,
	entry: &Entry,
) {
	for playlist in &entry.playlist {
		if let Some(set) = existing.get_mut(playlist) {
			if set.contains(&entry.s) {
				set.remove(&entry.s);
			}
		} else {
			if add.get(playlist).is_none() {
				add.insert(playlist.clone(), SyncEntry::default());
			}

			add.get_mut(playlist).unwrap().add.push(entry.s.clone());
		}
	}
}

fn remove_fs(existing: &HashSet<Source>, remove: &mut Vec<Source>) {
	for source in existing {
		remove.push(source.clone());
	}
}

fn remove_q(existing: &HashMap<Q, HashSet<Source>>, remove: &mut HashMap<Q, SyncEntry>) {
	for (q, set) in existing {
		if remove.get(q).is_none() {
			remove.insert(*q, SyncEntry::default());
		}

		for source in set {
			remove.get_mut(q).unwrap().remove.push(source.clone());
		}
	}
}

fn remove_playlist(
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
