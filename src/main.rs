mod entry;
mod env;
mod fetch;
mod filter;
mod parse;
mod playlist;
mod report;

use std::{future::IntoFuture, path::PathBuf};

use async_std::task::block_on;
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

	block_on(fetch::fetch(&sync.fs).into_future());

	// TODO: sync playlist

	Ok(())
}
