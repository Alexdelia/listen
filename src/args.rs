use std::path::PathBuf;

use clap::Parser;

use crate::declaration::Source;

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
		mbid: Source,
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
	/// resolve listenbrainz recommendations to matches and append new ones
	Recommend {
		/// listenbrainz.org username, or a musicbrainz.org artist MBID
		target: Option<String>,
		/// skip recommendations already listened to
		#[arg(short, long)]
		unlistened: bool,
		/// which listenbrainz recommendation to walk through
		#[arg(short, long, default_value = "all")]
		source: RecommendSource,
	},
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RecommendSource {
	/// alternate between every source that fits the target
	All,
	/// the raw collaborative filtering recording list
	CollaborativeFiltering,
	/// the most listened recording of an artist, needs an MBID
	#[value(name = "listenbrainz")]
	ListenBrainz,
	/// both kept weekly exploration playlists, last week first
	WeeklyExploration,
	/// the second to last weekly exploration playlist
	WeeklyExplorationLastWeek,
	/// the latest weekly exploration playlist
	WeeklyExplorationCurrentWeek,
}

pub fn parse() -> Args {
	Args::parse()
}
