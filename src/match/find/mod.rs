mod form;
mod matching;
mod query;
mod search;
mod text;

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};

use crate::streaming_source::StreamingSource;

use super::verify::{self, Info};

pub(super) struct Found {
	pub(super) url: String,
	pub(super) info: Info,
}

pub(super) async fn song(
	client: &MusicBrainzClient,
	recording: &Recording,
	title: &str,
	length: i64,
) -> hmerr::Result<Found> {
	let title_form = form::title(recording, title);
	let artist = form::artist(client, recording).await;
	let accepted = form::accepted_title(&title_form);

	for id in search::search(&query::build(&title_form, &artist))? {
		let Some(info) = verify::verify(&id)? else {
			continue;
		};

		if matching::is_match(&accepted, length, &info) {
			return Ok(Found {
				url: verify::watch(&id),
				info,
			});
		}
	}

	Err(ge!(format!(
		"{R}no exact {host} match for {B}{title}{D}",
		host = StreamingSource::YouTubeMusic.host()
	)))?
}

fn push_unique(out: &mut Vec<String>, s: String) {
	if !s.is_empty() && !out.contains(&s) {
		out.push(s);
	}
}
