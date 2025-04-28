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
use indicatif::{MultiProgress, ProgressStyle};

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
	println!();
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
	let mp = MultiProgress::new();

	let template = |title: &str, color: &str| {
		let title = format!("{title:>8}");
		ProgressStyle::with_template(
			&[
				&title,
				" {wide_bar:.",
				color,
				"/white} {pos:>4.bold.green}/{len:4.bold} {percent:>3.bold.green}% {elapsed:>3.bold.blue}|{eta:3.bold.magenta}",
			]
			.join(""),
		)
		.expect("failed to create progress style")
	};

	let pb_playlist = mp.add(indicatif::ProgressBar::new(total.playlist as u64));
	pb_playlist.set_style(template("playlist", "magenta"));
	if total.playlist > 0 {
		pb_playlist.tick();
	}

	let pb_remove = mp.add(indicatif::ProgressBar::new(total.remove as u64));
	pb_remove.set_style(template("remove", "red"));
	if total.remove > 0 {
		pb_remove.tick();
	}

	let pb_fetch = mp.add(indicatif::ProgressBar::new(total.fetch as u64));
	pb_fetch.set_style(template("fetch", "blue"));
	let pb_download = mp.add(indicatif::ProgressBar::new(total.fetch as u64));
	pb_download.set_style(template("download", "cyan"));
	let pb_metadata = mp.add(indicatif::ProgressBar::new(total.fetch as u64));
	pb_metadata.set_style(template("metadata", "green"));
	if total.fetch > 0 {
		pb_fetch.tick();
		pb_download.tick();
		pb_metadata.tick();
	}

	let mut err = vec![];

	while let Ok(status) = rx.recv_blocking() {
		match status.action {
			Action::FetchMusicBrainz => {
				pb_fetch.inc(1);
				pb_download.tick();
				pb_metadata.tick();
			}
			Action::FetchStreaming => {
				pb_fetch.tick();
				pb_download.inc(1);
				pb_metadata.tick();
			}
			Action::AddMetadata => {
				pb_fetch.tick();
				pb_download.tick();
				pb_metadata.inc(1);
			}
			Action::RemoveFile => pb_remove.inc(1),
			Action::SyncPlaylist => pb_playlist.inc(1),
		}

		if let Err(e) = status.status {
			eprintln!("{e}\n");
			err.push(e);
		}
	}

	if total.fetch > 0 {
		pb_fetch.finish();
		pb_download.finish();
		pb_metadata.finish();
	}
	if total.remove > 0 {
		pb_remove.finish();
	}
	if total.playlist > 0 {
		pb_playlist.finish();
	}

	if !err.is_empty() {
		eprint!("\n\nerrors:\n\n");
		for e in err {
			eprint!("{e}\n");
		}
		eprint!("\n\n\n");
	}
}
