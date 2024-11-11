mod channel;
mod entry;
mod env;
mod fetch;
mod filter;
mod parse;
mod playlist;
mod remove;
mod report;

use std::{future::IntoFuture, path::PathBuf, thread};

use async_std::{
	channel::{Receiver, Sender},
	task::block_on,
};
use channel::{Action, Status};
use clap::Parser;
use filter::{GroupedEntry, SyncEntry};
use hmerr::ioe;

#[derive(Parser)]
#[command(about)]
pub struct Args {
	/// path to the ron file where the listens are declared
	#[clap(default_value = "listen.ron")]
	path: PathBuf,
}

const MUSIC_BRAINZ_USER_AGENT: &str =
	"Alexdelia's personal declarative listen/0.1.0 ( https://github.com/Alexdelia/listen )";

fn main() -> hmerr::Result<()> {
	let args = Args::parse();

	env::load()?;

	let list = parse::parse(args.path)?;

	let sync = filter::sync(list)?;

	let remove = report::report(&sync);

	if remove {
		let yes = ux::ask_yn("do you want to proceed with this update?", true)
			.map_err(|e| ioe!("stdin", e))?;

		if !yes {
			return Ok(());
		}
	}

	let total = Count {
		fetch: sync.fs.add.len(),
		remove: sync.fs.remove.len(),
		playlist: sync.q.len() + sync.playlist.len(),
	};

	let (tx, rx) = async_std::channel::unbounded::<Status>();

	process(sync, tx);
	progress(total, rx);

	Ok(())
}

fn process(sync: GroupedEntry<SyncEntry>, tx: Sender<Status>) {
	let txc = tx.clone();
	thread::spawn(move || {
		block_on(fetch::fetch(&sync.fs.add, txc).into_future());
	});
	let txc = tx.clone();
	thread::spawn(move || {
		block_on(remove::remove(&sync.fs.remove, txc).into_future());
	});

	for (q, sync) in sync.q {
		let txc = tx.clone();
		thread::spawn(move || {
			block_on(playlist::sync::q(q, sync, txc).into_future());
		});
	}
	for (playlist, sync) in sync.playlist {
		let txc = tx.clone();
		thread::spawn(move || {
			block_on(playlist::sync::playlist(playlist, sync, txc).into_future());
		});
	}
}

#[derive(Default)]
struct Count {
	fetch: usize,
	remove: usize,
	playlist: usize,
}

fn progress(total: Count, rx: Receiver<Status>) {
	let mut sum = Count::default();
	let mut music_brainz = 0;
	let mut streaming = 0;

	while let Ok(status) = rx.recv_blocking() {
		match status.action {
			Action::FetchMusicBrainz => {
				music_brainz += 1;
			}
			Action::FetchStreaming => {
				streaming += 1;
			}
			Action::AddMetadata => {
				sum.fetch += 1;
			}
			Action::RemoveFile => {
				sum.remove += 1;
			}
			Action::SyncPlaylist => {
				sum.playlist += 1;
			}
		}

		println!(
			"fetch: {}/{} remove: {}/{} playlist: {}/{} musicbrainz: {} streaming: {}",
			sum.fetch,
			total.fetch,
			sum.remove,
			total.remove,
			sum.playlist,
			total.playlist,
			music_brainz,
			streaming
		);
	}
}
