mod args;
mod cache;
mod color;
mod declaration;
mod env;
mod library;
mod r#match;
mod music_brainz;
mod open;
mod outlier;
mod streaming_source;
mod sync;

use async_std::task::block_on;

use args::Command;

fn main() -> hmerr::Result<()> {
	let args = args::parse();

	if let Some(Command::Match { mbid }) = &args.command {
		return block_on(r#match::run(&args.path, mbid));
	}

	if let Some(Command::Outlier {
		username,
		refresh,
		interactive,
	}) = &args.command
	{
		return outlier::run(&args.path, username.as_deref(), *refresh, *interactive);
	}

	sync::run(&args.path, args.refresh_metadata)
}
