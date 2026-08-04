use ansi::abbrev::{B, D, R};
use hmerr::ge;
use musicbrainz_rs::{
	Browse, Fetch,
	entity::{artist::Artist, recording::Recording},
};

use crate::{declaration::Source, music_brainz};

const PAGE: u8 = 100;

pub(super) struct Catalogue {
	pub artist: String,
	pub recording: Vec<Source>,
}

pub(super) async fn catalogue(mbid: Source) -> hmerr::Result<Catalogue> {
	let client = music_brainz::client();
	let id = mbid.to_string();

	let artist = match Artist::fetch()
		.id(&id)
		.execute_with_client_async(&client)
		.await
	{
		Ok(artist) if confirmed(&artist.id, &id) => artist,
		Ok(_) => {
			return Err(
				not_an_artist(&client, &id, format!("{R}no artist under {B}{id}{D}")).await,
			);
		}
		Err(e) => {
			return Err(not_an_artist(
				&client,
				&id,
				format!("{R}failed to fetch artist {B}{id}{D}\n{e:#?}"),
			)
			.await);
		}
	};

	let mut recording = Vec::new();
	let mut offset = 0;
	loop {
		let page = Recording::browse()
			.by_artist(&id)
			.limit(PAGE)
			.offset(offset)
			.execute_with_client_async(&client)
			.await
			.map_err(|e| {
				ge!(format!(
					"{R}failed to browse the recordings of {B}{name}{D}\n{e:#?}",
					name = artist.name
				))
			})?;

		recording.extend(
			page.entities
				.iter()
				.filter_map(|entity| entity.id.parse::<Source>().ok()),
		);

		offset = offset.saturating_add(u16::from(PAGE));
		if page.entities.is_empty() || i32::from(offset) >= page.count {
			break;
		}
	}

	Ok(Catalogue {
		artist: artist.name,
		recording,
	})
}

fn confirmed(fetched: &str, id: &str) -> bool {
	fetched == id
}

async fn not_an_artist(
	client: &musicbrainz_rs::MusicBrainzClient,
	id: &str,
	failure: String,
) -> Box<dyn std::error::Error> {
	if Recording::fetch()
		.id(id)
		.execute_with_client_async(client)
		.await
		.is_ok_and(|recording| confirmed(&recording.id, id))
	{
		return ge!(
			format!("{R}{B}{id}{D}{R} is a recording, not an artist{D}"),
			h: "listenbrainz only recommends by listen count for an artist mbid"
		)
		.into();
	}

	ge!(failure).into()
}

#[cfg(test)]
mod tests {
	use super::*;

	const ID: &str = "beff21d3-88c7-4ee0-8b7a-40b6db22c6d7";

	#[test]
	fn an_entity_under_the_asked_mbid_is_confirmed() {
		let artist = Artist {
			id: ID.to_string(),
			name: "Pendulum".to_string(),
			..Default::default()
		};

		assert!(confirmed(&artist.id, ID));
	}

	#[test]
	fn a_musicbrainz_error_payload_confirms_nothing() {
		assert!(!confirmed(&Artist::default().id, ID));
		assert!(!confirmed("", ID));
	}
}
