use indicatif::ProgressBar;
use miette::miette;
use musicbrainz_rs::{Fetch, entity::recording::Recording};

use crate::{bar, declaration::Entry, library, music_brainz};

use super::super::tag;

pub(crate) async fn run(list: &[Entry]) -> miette::Result<()> {
	let client = music_brainz::client();

	let existing = list
		.iter()
		.filter(|entry| library::recording::path(entry.s).exists())
		.collect::<Vec<_>>();

	let pb = ProgressBar::new(existing.len() as u64);
	pb.set_style(bar::template("metadata", "green")?);
	pb.tick();

	let mut err = vec![];

	for entry in existing {
		let path = library::recording::path(entry.s);

		match Recording::fetch()
			.id(&entry.s.to_string())
			.with_aliases()
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
			Err(e) => err.push(miette!("failed to fetch {}\n{e}", entry.s)),
		}

		pb.inc(1);
	}

	pb.finish();

	if !err.is_empty() {
		eprint!("\n\nerrors:\n\n");
		for e in &err {
			eprintln!("{e:?}");
		}
	}

	Ok(())
}
