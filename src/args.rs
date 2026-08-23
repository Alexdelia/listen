use std::path::PathBuf;

use clap::Parser;

use crate::declaration::Source;

pub(crate) const POPULARITY_DAMP: f32 = 1.0 / 3.0;
pub(crate) const GRANULARITY: f64 = 1.5;

#[derive(Parser)]
#[command(about)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct Args {
	#[command(subcommand)]
	pub command: Option<Command>,

	/// path to the ron file where the listens are declared
	#[arg(default_value = "listen.ron")]
	pub path: PathBuf,

	/// refetch metadata from musicbrainz and rewrite tags for every downloaded recording
	#[arg(long)]
	pub refresh_metadata: bool,
}

#[derive(clap::Subcommand)]
pub(crate) enum Command {
	/// find the exact music.youtube.com match for a musicbrainz.org recording
	Match {
		/// musicbrainz.org recording MBID
		mbid: Source,
	},
	/// compare declared q against listen counts, off the unpacked dump or listenbrainz api
	Outlier {
		/// listenbrainz.org username, cached and optional after the first use
		username: Option<String>,
		/// refetch listen stats instead of using the cache
		#[arg(short, long)]
		refresh: bool,
		/// review each outlier and apply a new q to the ron file
		#[arg(short, long)]
		interactive: bool,
		/// read the capped listenbrainz api stats even when a listen dump is unpacked
		#[arg(long)]
		api: bool,
	},
	/// resolve listenbrainz recommendations to matches and append new ones
	Recommend {
		/// listenbrainz.org username, or a musicbrainz.org artist MBID
		target: Option<String>,
		/// skip recommendations already listened to
		#[arg(short, long)]
		unlistened: bool,
		/// which listenbrainz recommendation to walk through
		#[arg(short, long, value_enum, default_value_t = RecommendSource::All)]
		source: RecommendSource,
		/// in which order to walk the recordings of an artist
		#[arg(long, value_enum, default_value_t = RecommendSort::Popularity)]
		sort: RecommendSort,
		#[command(flatten)]
		island: IslandArg,
	},
	/// print the shell completion script for this command and its nix dev shell wrapper
	Completion {
		#[arg(value_enum, default_value_t = clap_complete::Shell::Bash)]
		shell: clap_complete::Shell,
	},
}

#[derive(clap::Args)]
pub(crate) struct IslandArg {
	/// how hard to damp how many listeners already play it
	#[arg(long, default_value_t = POPULARITY_DAMP)]
	pub popularity_damp: f32,
	/// island granularity, higher splits broad islands into narrower ones
	#[arg(long, default_value_t = GRANULARITY)]
	pub granularity: f64,
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
pub(crate) enum RecommendSource {
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
pub(crate) enum RecommendSort {
	/// the most listened recording first
	Popularity,
	/// the most recently released recording first, an undated one last
	Newest,
}

pub(crate) fn parse() -> Args {
	Args::parse()
}
