mod channel;
mod entry;
mod env;
mod fetch;
mod filter;
mod parse;
mod playlist;
mod report;

use std::{future::IntoFuture, os::linux::raw::stat, path::PathBuf, thread};

use async_std::task::block_on;
use channel::Status;
use clap::Parser;
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
	dbg!(&list);

	let sync = filter::sync(list)?;

	let remove = report::report(&sync);

	if remove {
		let yes = ux::ask_yn("do you want to proceed with this update?", true)
			.map_err(|e| ioe!("stdin", e))?;

		if !yes {
			return Ok(());
		}
	}

	let (tx, rx) = async_std::channel::unbounded::<Status>();

	// thread to fetch
	let txc = tx.clone();
	thread::spawn(move || {
		block_on(fetch::fetch(&sync.fs, txc).into_future());
	});

	// thread to sync playlist
	thread::spawn(move || {
		let res = block_on(
			tx.send(Status {
				action: channel::Action::SyncPlaylist,
				status: Ok(()),
			})
			.into_future(),
		);
		res.expect("failed to send sync playlist status");
	});

	// main thread to print the status
	while let Ok(status) = rx.recv_blocking() {
		println!("{:?}", status);
	}

	Ok(())
}
