mod mapping;
mod model;
mod output;
mod select;
mod vocabulary;

use std::collections::HashSet;

use ansi::{
	DIM,
	abbrev::{B, D, R, Y},
};
use hmerr::ge;
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
		eprintln!("{Y}no tag suggestion{D}\n{e}");
	}
}

fn run(recording: &Recording, url: Option<&str>) -> hmerr::Result<()> {
	let prefetch = Prefetch::cached()?;
	if let Err(e) = prefetch.prune() {
		eprintln!("{e}");
	}

	let Some(url) = url else {
		println!(
			"{B}{label}{D}  {DIM}no streaming url to listen to{D}",
			label = output::LABEL
		);
		return Ok(());
	};

	let source = recording.id.parse::<Source>().map_err(|e| {
		ge!(format!(
			"{R}{B}{id}{D}{R} is not an mbid{D}\n{e}",
			id = recording.id
		))
	})?;

	let spinner = bar::spinner("prefetch", "blue")?;
	let audio = prefetch.fetch(source, url);
	spinner.finish_and_clear();

	let Some(scores) = model::scores(&audio?)? else {
		return Ok(());
	};

	let genres = mapping::genres(&scores, &vocabulary::load()?);
	let suggestions = select::select(genres, &carried(recording));

	// TODO: checkbox selection of these genres (needs a checkbox prompt added to yahmrslib), then submit the chosen ones as MusicBrainz user tags (POST /ws/2/tag, OAuth scope "tag" next to "rating" in src/sync/rate/auth/login.rs)
	println!("{}", output::line(&suggestions));

	Ok(())
}

fn carried(recording: &Recording) -> HashSet<String> {
	let genres = recording.genres.iter().flatten().map(|genre| &genre.name);
	let tags = recording.tags.iter().flatten().map(|tag| &tag.name);

	genres
		.chain(tags)
		.map(|name| name.trim().to_lowercase())
		.collect()
}
