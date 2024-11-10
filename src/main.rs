mod entry;
mod parse;

use std::path::PathBuf;

use clap::Parser;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

#[derive(Parser)]
#[command(about)]
pub struct Args {
	/// path to the ron file where the listens are declared
	#[clap(default_value = "listen.ron")]
	path: PathBuf,
}

fn main() -> hmerr::Result<()> {
	let args = Args::parse();

	let list = parse::parse(args.path)?;

	list.par_iter().for_each(|entry| {
		println!("{:?}", entry);
	});

	Ok(())
}
