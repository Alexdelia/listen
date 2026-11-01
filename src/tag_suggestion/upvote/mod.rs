mod submit;

use console::style;

use crate::{
	declaration::Source,
	music_brainz::auth::{self, Client},
	prompt,
};

use super::{output, select::Suggestion};

const ASKED: &str = "genre to upvote";

enum Vote<'a> {
	Upvoted(Vec<&'a str>),
	Unchecked,
	NoLogin,
	Failed(miette::Report),
}

pub(super) fn offer(source: Source, suggestions: &[Suggestion]) -> miette::Result<()> {
	let Some(client) = askable(suggestions) else {
		print(suggestions, &[]);
		return Ok(());
	};

	let checked = prompt::check(ASKED, &output::items(suggestions))?
		.into_iter()
		.filter_map(|checked| suggestions.get(checked))
		.map(|suggestion| suggestion.name.as_str())
		.collect::<Vec<_>>();

	let vote = vote(&client, source, checked);
	print(suggestions, vote.upvoted());
	vote.say();

	Ok(())
}

fn askable(suggestions: &[Suggestion]) -> Option<Client> {
	if suggestions.is_empty() || !prompt::interactive() {
		return None;
	}

	auth::client()
}

impl Vote<'_> {
	fn upvoted(&self) -> &[&str] {
		match self {
			Self::Upvoted(upvoted) => upvoted,
			Self::Unchecked | Self::NoLogin | Self::Failed(_) => &[],
		}
	}

	fn say(&self) {
		match self {
			Self::Upvoted(_) | Self::Unchecked => {}
			Self::NoLogin => println!(
				"{}",
				style("upvote not submitted, no musicbrainz login").dim()
			),
			Self::Failed(e) => eprintln!("{}\n{e:?}", style("upvote not submitted").yellow()),
		}
	}
}

fn vote<'a>(client: &Client, source: Source, checked: Vec<&'a str>) -> Vote<'a> {
	if checked.is_empty() {
		return Vote::Unchecked;
	}

	let submitted = auth::acquire(client).and_then(|bearer| {
		bearer
			.map(|bearer| submit::submit(&bearer, source, &checked))
			.transpose()
	});

	match submitted {
		Ok(Some(())) => Vote::Upvoted(checked),
		Ok(None) => Vote::NoLogin,
		Err(e) => Vote::Failed(e),
	}
}

fn print(suggestions: &[Suggestion], upvoted: &[&str]) {
	for row in output::rows(suggestions, upvoted) {
		println!("{row}");
	}
}
