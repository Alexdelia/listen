use std::{
	collections::{HashMap, HashSet},
	fs,
};

use hmerr::ioe;

use crate::{
	entry::{Entry, Q, Source},
	playlist,
};

use super::GroupedEntry;

pub fn get() -> hmerr::Result<GroupedEntry<HashSet<Source>>> {
	let fs = fs()?;

	let m3u = m3u()?;

	Ok(GroupedEntry {
		fs,
		q: m3u.q,
		playlist: m3u.playlist,
	})
}

fn fs() -> hmerr::Result<HashSet<Source>> {
	let output = std::fs::read_dir(Entry::OUTPUT_DIR).map_err(|e| ioe!(Entry::OUTPUT_DIR, e))?;
	let mut existing = HashSet::<Source>::new();

	for entry in output {
		let entry = entry.map_err(|e| ioe!(Entry::OUTPUT_DIR, e))?;

		let path = entry.path();
		if !path.is_file() || path.extension().map(|ext| ext.to_str()) != Some(Some(Entry::EXT)) {
			continue;
		}

		let Some(source) = path.file_stem() else {
			continue;
		};

		let source = source.to_string_lossy();
		existing.insert(source.to_string());
	}

	Ok(existing)
}

#[derive(Default)]
struct M3uEntry {
	q: HashMap<Q, HashSet<Source>>,
	playlist: HashMap<String, HashSet<Source>>,
}

fn m3u() -> hmerr::Result<M3uEntry> {
	let mut ret = M3uEntry::default();

	let output =
		std::fs::read_dir(playlist::OUTPUT_DIR).map_err(|e| ioe!(playlist::OUTPUT_DIR, e))?;

	for entry in output {
		let entry = entry.map_err(|e| ioe!(playlist::OUTPUT_DIR, e))?;

		let path = entry.path();
		if !path.is_file() || path.extension().map(|ext| ext.to_str()) != Some(Some(playlist::EXT))
		{
			continue;
		}

		let Some(name) = path.file_stem() else {
			continue;
		};

		let list = playlist::parse_content(&fs::read_to_string(&path)?);

		let name = name.to_string_lossy();
		if name.starts_with(playlist::PREFIX) {
			let q = playlist::parse_q(&name)?;
			ret.q.insert(q, list);
		} else {
			ret.playlist.insert(name.to_string(), list);
		}
	}

	Ok(ret)
}
