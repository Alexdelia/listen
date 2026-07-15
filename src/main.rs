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

use std::path::PathBuf;

use async_std::task::block_on;
use clap::Parser;

#[derive(Parser)]
#[command(about)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Args {
	#[command(subcommand)]
	command: Option<Command>,

	/// path to the ron file where the listens are declared
	#[clap(default_value = "listen.ron")]
	path: PathBuf,

	/// refetch metadata from musicbrainz and rewrite tags for every downloaded recording
	#[arg(long)]
	refresh_metadata: bool,
}

#[derive(clap::Subcommand)]
enum Command {
	/// find the exact music.youtube.com match for a musicbrainz.org recording
	Match {
		/// musicbrainz.org recording MBID
		mbid: String,
	},
	/// compare declared q against listenbrainz listen counts to surface outliers
	Outlier {
		/// listenbrainz.org username, cached and optional after the first use
		username: Option<String>,
		/// refetch listen stats instead of using the cache
		#[arg(short, long)]
		refresh: bool,
		/// review each outlier and apply a new q to the ron file
		#[arg(short, long)]
		interactive: bool,
	},
}

fn main() -> hmerr::Result<()> {
	let args = Args::parse();

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
