mod entry;
mod env;
mod fetch;
mod parse;

use std::{
	future::{self, IntoFuture},
	path::PathBuf,
};

use async_std::task::block_on;
use clap::Parser;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use soundcloud::Client;

#[derive(Parser)]
#[command(about)]
pub struct Args {
	/// path to the ron file where the listens are declared
	#[clap(default_value = "listen.ron")]
	path: PathBuf,
}

fn main() -> hmerr::Result<()> {
	let args = Args::parse();

	env::load()?;

	let list = parse::parse(args.path)?;

	let sc_client = Client::new(&env::get(env::Var::SoundcloudClientId)?);
	let res = sc_client.resolve("https://soundcloud.com/feintdnb/words-feat-laura-brehm");

	block_on(async {
		dbg!(res.await);
	});

	list.par_iter().for_each(fetch::fetch);

	Ok(())
}
