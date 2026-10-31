mod mapping;
mod model;
mod output;
mod select;
mod upvote;
mod vocabulary;

use std::collections::HashSet;

use console::style;
use miette::miette;
use musicbrainz_rs::entity::recording::Recording;

use crate::{
	bar,
	declaration::Source,
	env::{self, Var},
	prefetch::Prefetch,
};

pub(crate) fn suggest(recording: &Recording, url: Option<&str>) {
	if !env::enabled(Var::ComputeTagSuggestion) {
		return;
	}

	if let Err(e) = run(recording, url) {
		eprintln!("{}\n{e:?}", style("no tag suggestion").yellow());
	}
}

fn run(recording: &Recording, url: Option<&str>) -> miette::Result<()> {
	let Some(url) = url else {
		println!(
			"{}  {}",
			style(&output::LABEL).bold(),
			style("no streaming url to listen to").dim()
		);
		return Ok(());
	};

	let source = recording
		.id
		.parse::<Source>()
		.map_err(|e| miette!("{id} is not an mbid\n{e}", id = recording.id))?;

	let spinner = bar::spinner("prefetch", "blue")?;
	let audio = Prefetch::cached()?.fetch(source, url);
	spinner.finish_and_clear();

	let Some(scores) = model::scores(&audio?)? else {
		return Ok(());
	};

	let genres = mapping::genres(&scores, &vocabulary::load()?);
	let suggestions = select::select(genres, &carried(recording));

	upvote::offer(source, &suggestions)
}

fn carried(recording: &Recording) -> HashSet<String> {
	let genres = recording.genres.iter().flatten().map(|genre| &genre.name);
	let tags = recording.tags.iter().flatten().map(|tag| &tag.name);

	genres
		.chain(tags)
		.map(|name| name.trim().to_lowercase())
		.collect()
}
