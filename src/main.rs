mod entry;
mod env;
// mod fetch;
mod filter;
mod parse;
mod playlist;
mod report;

use std::path::PathBuf;

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

	let remove = report::report(sync);

	if remove {
		let yes = ux::ask_yn("do you want to proceed with this update?", true)
			.map_err(|e| ioe!("stdin", e))?;

		if !yes {
			return Ok(());
		}
	}

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

	// let res: Release = Release::fetch()
	// 	.id("e56934aa-a110-4820-aad9-4ca825c71b7f")
	// 	.with_url_relations()
	// 	.execute()?;
	// dbg!(&res);

	/*
	let list = parse::parse(args.path)?;

	list.par_iter().for_each(fetch::fetch);
	*/

	Ok(())
}
