mod submit;

use console::style;

use crate::{declaration::Source, music_brainz::auth, prompt};

use super::{output, select::Suggestion};

const ASKED: &str = "genre to upvote";

pub(super) fn offer(source: Source, suggestions: &[Suggestion]) -> miette::Result<()> {
	let client = auth::client().filter(|_| !suggestions.is_empty());
	let Some(client) = client else {
		for row in output::rows(suggestions) {
			println!("{row}");
		}
		return Ok(());
	};

	let upvoted = prompt::check(ASKED, &output::items(suggestions))?
		.into_iter()
		.filter_map(|checked| suggestions.get(checked))
		.map(|suggestion| suggestion.name.as_str())
		.collect::<Vec<_>>();

	if upvoted.is_empty() {
		return Ok(());
	}

	let Some(bearer) = auth::acquire(&client)? else {
		return Ok(());
	};

	submit::submit(&bearer, source, &upvoted)?;

	println!(
		"{}  {}",
		style("upvoted").bold().green(),
		upvoted.join(", ")
	);

	Ok(())
}
