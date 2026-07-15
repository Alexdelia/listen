use ansi::abbrev::{B, D, R};
use indicatif::{ProgressBar, ProgressStyle};
use musicbrainz_rs::{Fetch, MusicBrainzClient, entity::recording::Recording};

use crate::{declaration::Entry, library, music_brainz};

use super::super::tag;

const TEMPLATE: &str =
	"metadata {wide_bar:.green/white} {pos:>4.bold.green}/{len:4.bold} {percent:>3.bold.green}%";

pub async fn run(list: &[Entry]) -> hmerr::Result<()> {
	let client = MusicBrainzClient::new(music_brainz::USER_AGENT);

	let existing = list
		.iter()
		.filter(|entry| library::recording::path(entry.s).exists())
		.collect::<Vec<_>>();

	let pb = ProgressBar::new(existing.len() as u64);
	pb.set_style(
		ProgressStyle::with_template(TEMPLATE)
			.map_err(|e| format!("failed to create progress style\n{e}"))?,
	);

	let mut err = vec![];

	for entry in existing {
		let path = library::recording::path(entry.s);

		match Recording::fetch()
			.id(&entry.s.to_string())
			.with_artists()
			.with_genres()
			.with_tags()
			.execute_with_client_async(&client)
			.await
		{
			Ok(recording) => {
				if let Err(e) = tag::write(&path, &recording) {
					err.push(e);
				}
			}
			Err(e) => err.push(format!("{R}failed to fetch {B}{s}{D}\n{e}", s = entry.s)),
		}

		pb.inc(1);
	}

	pb.finish();

	if !err.is_empty() {
		eprint!("\n\nerrors:\n\n");
		for e in &err {
			eprintln!("{e}");
		}
	}

	Ok(())
}
