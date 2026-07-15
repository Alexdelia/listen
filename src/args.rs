use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(about)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Args {
	#[command(subcommand)]
	pub command: Option<Command>,

	/// path to the ron file where the listens are declared
	#[clap(default_value = "listen.ron")]
	pub path: PathBuf,

	/// refetch metadata from musicbrainz and rewrite tags for every downloaded recording
	#[arg(long)]
	pub refresh_metadata: bool,
}

#[derive(clap::Subcommand)]
pub enum Command {
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

pub fn parse() -> Args {
	Args::parse()
}
