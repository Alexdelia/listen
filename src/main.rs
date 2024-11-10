mod entry;
mod env;
mod fetch;
mod parse;

use std::path::PathBuf;

use clap::Parser;
use musicbrainz_rs_nova::entity::{recording::Recording, release::Release};
use musicbrainz_rs_nova::Fetch;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

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

	musicbrainz_rs_nova::config::set_user_agent(MUSIC_BRAINZ_USER_AGENT);

	// let res = Recording::fetch()
	// 	.id("7afff9fa-0de6-4e77-a210-0cbb78f56c2d")
	// 	// .with_artists()
	// 	// .with_genres()
	// 	// .with_tags()
	// 	// .with_releases()
	// 	// .with_medias()
	// 	// .with_work_level_relations()
	// 	// .with_work_relations()
	// 	.with_url_relations()
	// 	.execute()?;
	// dbg!(&res);

	let res: Release = Release::fetch()
		.id("e56934aa-a110-4820-aad9-4ca825c71b7f")
		.with_url_relations()
		.execute()?;
	dbg!(&res);

	/*
	let list = parse::parse(args.path)?;

	list.par_iter().for_each(fetch::fetch);
	*/

	Ok(())
}
