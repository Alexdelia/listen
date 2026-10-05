use ansi::abbrev::{D, R};
use hmerr::ge;

use crate::{cache, declaration::Source};

pub(super) enum Target {
	Username(String),
	Artist(Source),
}

pub(super) fn resolve(target: Option<&str>) -> hmerr::Result<Target> {
	match target.map(parse) {
		Some(Target::Artist(mbid)) => Ok(Target::Artist(mbid)),
		Some(Target::Username(username)) => {
			Ok(Target::Username(cache::username::resolve(Some(&username))?))
		}
		None => Ok(Target::Username(cache::username::resolve(None)?)),
	}
}

pub(super) fn known(target: Option<&str>) -> hmerr::Result<Target> {
	if let Some(target) = target {
		return Ok(parse(target));
	}

	cache::username::read()?
		.map(Target::Username)
		.ok_or_else(|| {
			ge!(
				format!("{R}no listenbrainz username remembered{D}"),
				h: "name one: recommend --json <username>, or run once without --json"
			)
			.into()
		})
}

fn parse(target: &str) -> Target {
	target
		.parse()
		.map_or_else(|_| Target::Username(target.to_string()), Target::Artist)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn an_mbid_is_an_artist() {
		assert!(matches!(
			parse("beff21d3-88c7-4ee0-8b7a-40b6db22c6d7"),
			Target::Artist(mbid) if mbid.to_string() == "beff21d3-88c7-4ee0-8b7a-40b6db22c6d7"
		));
	}

	#[test]
	fn anything_else_is_a_username() {
		assert!(matches!(
			parse("alexdelia"),
			Target::Username(username) if username == "alexdelia"
		));
	}

	#[test]
	fn a_truncated_mbid_is_not_an_artist() {
		assert!(matches!(parse("beff21d3-88c7"), Target::Username(_)));
	}

	#[test]
	fn a_known_artist_target_needs_no_cache() {
		assert!(matches!(
			known(Some("beff21d3-88c7-4ee0-8b7a-40b6db22c6d7")),
			Ok(Target::Artist(_))
		));
	}

	#[test]
	fn a_known_username_is_taken_as_given() {
		assert!(matches!(
			known(Some("alexdelia")),
			Ok(Target::Username(username)) if username == "alexdelia"
		));
	}
}
