use musicbrainz_rs::{
	Fetch, MusicBrainzClient,
	entity::{alias::Alias, artist::Artist, artist_credit::ArtistCredit, recording::Recording},
};

use super::{
	push_unique,
	text::{normalize_title, romanize},
};

pub(super) fn title(recording: &Recording, title: &str) -> Vec<String> {
	with_romaji(std::iter::once(title.to_string()).chain(alias_name(recording.aliases.as_deref())))
}

pub(super) async fn artist(client: &MusicBrainzClient, recording: &Recording) -> Vec<Vec<String>> {
	let mut out = Vec::new();

	for credit in recording.artist_credit.as_deref().unwrap_or_default() {
		out.push(credit_form(client, credit).await);
	}

	out
}

async fn credit_form(client: &MusicBrainzClient, credit: &ArtistCredit) -> Vec<String> {
	let mut name = vec![credit.artist.name.clone()];

	if let Ok(artist) = Artist::fetch()
		.id(&credit.artist.id)
		.with_aliases()
		.execute_with_client_async(client)
		.await
	{
		name.extend(alias_name(artist.aliases.as_deref()));
	}

	with_romaji(name)
}

pub(super) fn accepted_title(title_form: &[String]) -> Vec<String> {
	let mut out = Vec::new();

	for form in title_form {
		push_unique(&mut out, normalize_title(form));
	}

	out
}

fn alias_name(alias: Option<&[Alias]>) -> Vec<String> {
	alias
		.into_iter()
		.flatten()
		.map(|a| a.name.clone())
		.collect()
}

fn with_romaji(form: impl IntoIterator<Item = String>) -> Vec<String> {
	let mut out = Vec::new();

	for f in form {
		if let Some(romaji) = romanize(&f) {
			push_unique(&mut out, romaji);
		}
		push_unique(&mut out, f);
	}

	out
}
