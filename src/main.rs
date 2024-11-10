mod entry;
mod parse;

use std::path::PathBuf;

use clap::Parser;

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

	dbg!(list);

	Ok(())
}
