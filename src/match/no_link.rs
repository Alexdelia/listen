use std::{ops::ControlFlow, path::Path};

use ansi::abbrev::{D, G, R};
use hmerr::ioe;
use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};
use ux::AskKey;

use super::{Prompt, find, open, output};

pub(super) async fn run(
	client: &MusicBrainzClient,
	recording: &Recording,
	title: &str,
	length: i64,
	path: &Path,
	mbid: &str,
	prompt: Prompt,
) -> hmerr::Result<ControlFlow<()>> {
	let found = find::song(client, recording, title, length).await?;

	output::found(&found.info, length);
	open::open(&found.url)?;

	match confirm(prompt)? {
		Answer::Skip => return Ok(ControlFlow::Continue(())),
		Answer::Quit => return Ok(ControlFlow::Break(())),
		Answer::Accept => {}
	}

	output::musicbrainz(mbid, &found.url)?;
	output::entry(path, mbid)?;

	Ok(ControlFlow::Continue(()))
}

enum Answer {
	Accept,
	Skip,
	Quit,
}

fn confirm(prompt: Prompt) -> hmerr::Result<Answer> {
	match prompt {
		Prompt::Confirm => Ok(
			if ux::ask_yn("does this song match", true).map_err(|e| ioe!("stdin", e))? {
				Answer::Accept
			} else {
				Answer::Skip
			},
		),
		Prompt::Review => {
			let answer =
				ux::ask("does this song match", &key(), Some('q')).map_err(|e| ioe!("stdin", e))?;

			Ok(match answer {
				'y' => Answer::Accept,
				'n' => Answer::Skip,
				_ => Answer::Quit,
			})
		}
	}
}

fn key() -> [AskKey; 3] {
	[
		AskKey::new('y', Some("match"), true, Some(G)),
		AskKey::new('n', Some("skip"), true, Some(D)),
		AskKey::new('q', Some("quit"), true, Some(R)),
	]
}
