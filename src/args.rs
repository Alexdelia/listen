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
		/// in which order to walk the recordings of an artist
		#[arg(long, default_value = "popularity")]
		sort: RecommendSort,
		#[command(flatten)]
		island: IslandArg,
	},
	/// print the shell completion script for this command and its nix dev shell wrapper
	Completion {
		#[arg(default_value = "bash")]
		shell: clap_complete::Shell,
	},
}

#[derive(clap::Args)]
pub struct IslandArg {
	/// how hard to damp popularity, 0 keeps scene hits, above 0.8 reaches the untrustworthy [0.6]
	#[arg(long)]
	pub alpha: Option<f32>,
	/// island granularity, higher splits broad islands [1.0]
	#[arg(long)]
	pub resolution: Option<f32>,
	/// pin the stream to the islands whose name contains this
	#[arg(long)]
	pub island: Option<String>,
	/// ask after every recommendation whether the next one stays on the same island
	#[arg(long)]
	pub ask: bool,
	/// build one island out of these declared recordings instead of detecting islands
	#[arg(long)]
	pub seed: Vec<Source>,
	/// build one island out of every declared recording tagged with this local mp3 genre
	#[arg(long)]
	pub genre: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RecommendSource {
	/// alternate between every source that fits the target
	All,
	/// taste islands from the local listenbrainz index, needs a built index
	Island,
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

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RecommendSort {
	/// the most listened recording first
	Popularity,
	/// the most recently released recording first, an undated one last
	Newest,
}

pub fn parse() -> Args {
	Args::parse()
}
