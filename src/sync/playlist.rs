use std::{
	collections::{HashMap, HashSet},
	fs,
	path::Path,
};

use async_std::channel::Sender;
use hmerr::ioe;

use crate::{
	declaration::{Q, Source},
	library::{self, playlist::parse_content, tag::Sort},
};

use super::{
	channel::{Action, Status, report},
	filter::SyncEntry,
};

type SortName = HashMap<Source, Sort>;

pub(super) async fn all(
	q: HashMap<Q, SyncEntry>,
	playlist: HashMap<String, SyncEntry>,
	tx: Sender<Status>,
) {
	let mut sort_name = SortName::new();

	for (q, sync_entry) in q {
		synced(
			&library::playlist::q_path(q),
			sync_entry,
			&mut sort_name,
			&tx,
		)
		.await;
	}

	for (playlist, sync_entry) in playlist {
		synced(
			&library::playlist::path(&playlist),
			sync_entry,
			&mut sort_name,
			&tx,
		)
		.await;
	}
}

async fn synced(path: &Path, sync_entry: SyncEntry, sort_name: &mut SortName, tx: &Sender<Status>) {
	let status = sync(path, sync_entry, sort_name).map_err(|e| e.to_string());

	report(tx, Action::SyncPlaylist, status).await;
}

fn sync(path: &Path, sync: SyncEntry, sort_name: &mut SortName) -> hmerr::Result<()> {
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

	let content = content(&library::playlist::header(&previous), set, sort_name)?;

	if content == previous {
		return Ok(());
	}

	fs::write(path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn content(header: &str, set: HashSet<Source>, sort_name: &mut SortName) -> hmerr::Result<String> {
	let recording_path = std::env::current_dir()
		.map_err(|e| ioe!("current_dir", e))?
		.join(library::recording::DIR);
	let recording_path = recording_path
		.canonicalize()
		.map_err(|e| ioe!(recording_path.to_string_lossy(), e))?;

	let mut list = set
		.into_iter()
		.map(|source| {
			(
				sort_name
					.entry(source)
					.or_insert_with(|| library::tag::sort(source))
					.clone(),
				source,
			)
		})
		.collect::<Vec<_>>();
	list.sort();

	let body = list
		.into_iter()
		.map(|(_, entry)| {
			recording_path
				.join(entry.to_string())
				.with_extension(library::recording::EXT)
				.to_string_lossy()
				.to_string()
		})
		.collect::<Vec<_>>()
		.join("\n");

	Ok(if header.is_empty() {
		body
	} else {
		format!("{header}\n{body}")
	})
}
