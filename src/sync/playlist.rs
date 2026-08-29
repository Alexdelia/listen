use std::{
	collections::{HashMap, HashSet},
	fs,
	path::Path,
};

use async_std::channel::Sender;
use hmerr::ioe;

use crate::{
	declaration::{Q, Source},
	library::{self, playlist::parse_content},
};

use super::{
	channel::{Action, Status, report},
	filter::SyncEntry,
};

pub(super) async fn all(
	q: HashMap<Q, SyncEntry>,
	playlist: HashMap<String, SyncEntry>,
	tx: Sender<Status>,
) {
	for (q, sync_entry) in q {
		synced(&library::playlist::q_path(q), sync_entry, &tx).await;
	}

	for (playlist, sync_entry) in playlist {
		synced(&library::playlist::path(&playlist), sync_entry, &tx).await;
	}
}

async fn synced(path: &Path, sync_entry: SyncEntry, tx: &Sender<Status>) {
	let status = sync(path, sync_entry).map_err(|e| e.to_string());

	report(tx, Action::SyncPlaylist, status).await;
}

fn sync(path: &Path, sync: SyncEntry) -> hmerr::Result<()> {
	let previous = if path.exists() {
		fs::read_to_string(path).map_err(|e| ioe!(path.to_string_lossy(), e))?
	} else {
		String::new()
	};

	let mut set = parse_content(&previous);

	for entry in sync.add {
		set.insert(entry);
	}
	for entry in sync.remove {
		set.remove(&entry);
	}

	if set.is_empty() {
		if path.exists() {
			fs::remove_file(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
		}
		return Ok(());
	}

	let content = content(set)?;

	if content == previous {
		return Ok(());
	}

	fs::write(path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn content(set: HashSet<Source>) -> hmerr::Result<String> {
	let recording_path = std::env::current_dir()
		.map_err(|e| ioe!("current_dir", e))?
		.join(library::recording::DIR);
	let recording_path = recording_path
		.canonicalize()
		.map_err(|e| ioe!(recording_path.to_string_lossy(), e))?;

	let mut list = set.into_iter().collect::<Vec<_>>();
	list.sort_by_cached_key(|source| (library::tag::sort(*source), *source));

	Ok(list
		.into_iter()
		.map(|entry| {
			recording_path
				.join(entry.to_string())
				.with_extension(library::recording::EXT)
				.to_string_lossy()
				.to_string()
		})
		.collect::<Vec<_>>()
		.join("\n"))
}
