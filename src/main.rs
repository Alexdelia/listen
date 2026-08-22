mod alias;
mod args;
mod cache;
mod completion;
mod declaration;
mod env;
mod format;
mod library;
mod listen_brainz;
mod r#match;
mod meta_brainz;
mod music_brainz;
mod open;
mod outlier;
mod recommend;
mod streaming_source;
mod sync;

use async_std::task::block_on;

use args::Command;

fn main() -> hmerr::Result<()> {
	env::read();

	let args = args::parse();

	if let Some(Command::Completion { shell }) = &args.command {
		completion::run(*shell);
		return Ok(());
	}

	if let Some(Command::Match { mbid }) = &args.command {
		block_on(r#match::run(&args.path, &mbid.to_string(), false))?;
		return Ok(());
	}

	if let Some(Command::Outlier {
		username,
		refresh,
		interactive,
		api,
	}) = &args.command
	{
		return outlier::run(
			&args.path,
			username.as_deref(),
			*refresh,
			*interactive,
			*api,
		);
	}

	if let Some(Command::Recommend {
		target,
		unlistened,
		source,
		sort,
		island,
	}) = &args.command
	{
		return block_on(recommend::run(
			&args.path,
			target.as_deref(),
			*unlistened,
			*source,
			*sort,
			island,
		));
	}

	sync::run(&args.path, args.refresh_metadata)
}
