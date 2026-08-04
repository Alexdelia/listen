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

fn parse(target: &str) -> Target {
	match target.parse() {
		Ok(mbid) => Target::Artist(mbid),
		Err(_) => Target::Username(target.to_string()),
	}
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
}
