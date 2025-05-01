use std::{collections::HashSet, fs, path::Path};

use async_std::channel::Sender;
use hmerr::ioe;

use crate::{
	channel::{Action, Status},
	entry::{Entry, Q},
	filter::SyncEntry,
};

use super::{parse_content, playlist_path, q_path};

pub async fn q(q: Q, sync_entry: SyncEntry, tx: Sender<Status>) {
	let path = q_path(q);

	match sync(&path, sync_entry) {
		Ok(_) => tx.send(Status {
			action: Action::SyncPlaylist,
			status: Ok(()),
		}),
		Err(e) => tx.send(Status {
			action: Action::SyncPlaylist,
			status: Err(e.to_string()),
		}),
	}
	.await
	.expect("failed to send sync playlist status");
}

pub async fn playlist(playlist: String, sync_entry: SyncEntry, tx: Sender<Status>) {
	let path = playlist_path(&playlist);

	match sync(&path, sync_entry) {
		Ok(_) => tx.send(Status {
			action: Action::SyncPlaylist,
			status: Ok(()),
		}),
		Err(e) => tx.send(Status {
			action: Action::SyncPlaylist,
			status: Err(e.to_string()),
		}),
	}
	.await
	.expect("failed to send sync playlist status");
}

fn sync<P>(path: P, sync: SyncEntry) -> hmerr::Result<()>
where
	P: AsRef<Path>,
{
	let path = path.as_ref();

	let mut set = if path.exists() {
		parse_content(&fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?)
	} else {
		HashSet::new()
	};

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

	let mut list = set.into_iter().collect::<Vec<_>>();
	list.sort();

	let recording_path = std::env::current_dir()
		.map_err(|e| ioe!("current_dir", e))?
		.join(Entry::OUTPUT_DIR);
	let recording_path = recording_path
		.canonicalize()
		.map_err(|e| ioe!(recording_path.to_string_lossy(), e))?;
	fs::write(
		path,
		list.into_iter()
			.map(|entry| {
				recording_path
					.join(entry)
					.with_extension(Entry::EXT)
					.to_string_lossy()
					.to_string()
			})
			.collect::<Vec<_>>()
			.join("\n"),
	)
	.map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}
