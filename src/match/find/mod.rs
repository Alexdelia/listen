mod form;
mod matching;
mod miss;
mod query;
mod search;
mod text;

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};

use crate::streaming_source::StreamingSource;

use self::miss::Miss;
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
	mbid: &str,
) -> hmerr::Result<Found> {
	let title_form = form::title(recording, title);
	let artist = form::artist(client, recording).await;
	let accepted = form::accepted_title(&title_form);

	let mut miss = Vec::new();

	for id in search::search(&query::build(&title_form, &artist))? {
		let Some(info) = verify::verify(&id)? else {
			continue;
		};

		match matching::check(&accepted, length, &info) {
			None => {
				return Ok(Found {
					url: verify::watch(&id),
					info,
				});
			}
			Some(reason) => miss.push(Miss {
				url: verify::watch(&id),
				reason,
			}),
		}
	}

	Err(ge!(format!(
		"{R}no exact {host} match for {B}{title}{D}\nhttps://musicbrainz.org/recording/{mbid}{block}",
		host = StreamingSource::YouTubeMusic.host(),
		block = miss::block(miss),
	)))?
}

fn push_unique(out: &mut Vec<String>, s: String) {
	if !s.is_empty() && !out.contains(&s) {
		out.push(s);
	}
}
