use std::collections::BTreeSet;

use ansi::abbrev::{B, D, R};
use hmerr::{GenericError, ge};

use crate::declaration::Source;

use crate::library::tag::GENRE_SEPARATOR;

use super::{super::seed::Library, Request};

pub(super) fn island(
	library: &Library,
	genre: &[Vec<String>],
	request: &Request,
) -> hmerr::Result<(String, Vec<usize>)> {
	let mut member = BTreeSet::new();

	for mbid in &request.recording {
		match library.seed.iter().position(|seed| seed.mbid == *mbid) {
			Some(index) => {
				member.insert(index);
			}
			None if library.declared.contains(mbid) => return Err(unsupported(*mbid).into()),
			None => return Err(undeclared(*mbid).into()),
		}
	}

	for token in &request.genre {
		let token = token.to_lowercase();
		let matched: Vec<usize> = genre
			.iter()
			.enumerate()
			.filter(|(_, genre)| genre.contains(&token))
			.map(|(index, _)| index)
			.collect();

		if matched.is_empty() {
			return Err(no_genre(&token).into());
		}

		member.extend(matched);
	}

	Ok((name(request, member.len()), member.into_iter().collect()))
}

fn name(request: &Request, member: usize) -> String {
	if request.genre.is_empty() {
		return format!("{member} declared");
	}

	request
		.genre
		.iter()
		.map(|genre| genre.to_lowercase())
		.collect::<Vec<_>>()
		.join(GENRE_SEPARATOR)
}

fn undeclared(mbid: Source) -> GenericError {
	ge!(
		format!("{R}recording {B}{mbid}{D}{R} is not declared{D}"),
		h: "an island seed carries the q it is weighted by, so it has to be in the declaration"
	)
}

fn unsupported(mbid: Source) -> GenericError {
	ge!(
		format!("{R}recording {B}{mbid}{D}{R} has no listener in the index{D}"),
		h: "nobody in the index has played it, so it cannot reach a cohort"
	)
}

fn no_genre(token: &str) -> GenericError {
	ge!(
		format!("{R}no declared recording is tagged {B}{token}{D}"),
		h: "the genre comes from the local mp3 id3 tag, not from musicbrainz"
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	use crate::recommend::island::seed::{Listener, Seed};

	fn mbid(nibble: u8) -> Source {
		Source::from_bytes([nibble; 16])
	}

	fn library(supported: &[u8], declared: &[u8]) -> Library {
		Library {
			seed: supported
				.iter()
				.map(|nibble| Seed {
					mbid: mbid(*nibble),
					q: 2,
					listener: vec![Listener {
						user: 1,
						weight: 1.0,
					}],
				})
				.collect(),
			user: vec![0, 1],
			declared: declared.iter().map(|nibble| mbid(*nibble)).collect(),
		}
	}

	fn genre(token: &[&[&str]]) -> Vec<Vec<String>> {
		token
			.iter()
			.map(|token| token.iter().map(|token| (*token).to_string()).collect())
			.collect()
	}

	fn request(recording: &[u8], genre: &[&str]) -> Request {
		Request {
			recording: recording.iter().map(|nibble| mbid(*nibble)).collect(),
			genre: genre.iter().map(|genre| (*genre).to_string()).collect(),
		}
	}

	#[test]
	fn the_requested_recordings_become_one_island() {
		let library = library(&[1, 2, 3], &[1, 2, 3]);
		let genre = genre(&[&[], &[], &[]]);

		let (_, member) = island(&library, &genre, &request(&[1, 3], &[])).unwrap_or_default();

		assert_eq!(member, vec![0, 2]);
	}

	#[test]
	fn a_genre_collects_every_recording_tagged_with_it() {
		let library = library(&[1, 2, 3], &[1, 2, 3]);
		let genre = genre(&[&["touhou"], &["pop"], &["touhou", "metal"]]);

		let (_, member) = island(&library, &genre, &request(&[], &["touhou"])).unwrap_or_default();

		assert_eq!(member, vec![0, 2]);
	}

	#[test]
	fn a_genre_matches_whatever_case_it_is_asked_in() {
		let library = library(&[1, 2], &[1, 2]);
		let genre = genre(&[&["touhou"], &["pop"]]);

		let (_, member) = island(&library, &genre, &request(&[], &["TOUHOU"])).unwrap_or_default();

		assert_eq!(member, vec![0]);
	}

	#[test]
	fn seeds_and_genres_union_without_repeating_a_member() {
		let library = library(&[1, 2, 3], &[1, 2, 3]);
		let genre = genre(&[&["touhou"], &["pop"], &[]]);

		let (_, member) =
			island(&library, &genre, &request(&[1, 3], &["touhou"])).unwrap_or_default();

		assert_eq!(member, vec![0, 2]);
	}

	#[test]
	fn an_undeclared_seed_is_refused_and_says_so() {
		let library = library(&[1], &[1]);
		let said = island(&library, &genre(&[&[]]), &request(&[9], &[]))
			.err()
			.map(|e| e.to_string())
			.unwrap_or_default();

		assert!(said.contains("not declared"), "{said}");
	}

	#[test]
	fn a_declared_seed_nobody_listens_to_is_refused_separately() {
		let library = library(&[1], &[1, 7]);
		let said = island(&library, &genre(&[&[]]), &request(&[7], &[]))
			.err()
			.map(|e| e.to_string())
			.unwrap_or_default();

		assert!(said.contains("no listener"), "{said}");
	}

	#[test]
	fn an_unknown_genre_is_refused() {
		let library = library(&[1], &[1]);
		let said = island(&library, &genre(&[&["pop"]]), &request(&[], &["touhou"]))
			.err()
			.map(|e| e.to_string())
			.unwrap_or_default();

		assert!(said.contains("touhou"), "{said}");
	}

	#[test]
	fn the_genre_request_names_the_island() {
		let library = library(&[1, 2], &[1, 2]);
		let genre = genre(&[&["touhou"], &["eurobeat"]]);

		let (name, _) =
			island(&library, &genre, &request(&[], &["TOUHOU", "eurobeat"])).unwrap_or_default();

		assert_eq!(name, "touhou / eurobeat");
	}

	#[test]
	fn a_seed_only_request_is_named_after_its_size() {
		let library = library(&[1, 2], &[1, 2]);
		let genre = genre(&[&[], &[]]);

		let (name, _) = island(&library, &genre, &request(&[1, 2], &[])).unwrap_or_default();

		assert_eq!(name, "2 declared");
	}
}
