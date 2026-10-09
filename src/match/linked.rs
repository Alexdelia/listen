use std::collections::HashSet;

use super::{find::Found, redirect, verify};

pub(super) enum Linked {
	Song(Found),
	Replacement(Found),
	Video,
	Dead,
}

enum Seen {
	Song(verify::Info),
	Video,
	Gone { replacement: Option<String> },
}

pub(super) fn resolve(ids: Vec<String>) -> hmerr::Result<Linked> {
	resolve_with(ids, see)
}

fn see(id: &str) -> hmerr::Result<Seen> {
	Ok(match verify::verify(id)? {
		Some(info) if info.is_song() => Seen::Song(info),
		Some(_video) => Seen::Video,
		None => Seen::Gone {
			replacement: redirect::resolve(id)?,
		},
	})
}

fn resolve_with<F>(ids: Vec<String>, mut see: F) -> hmerr::Result<Linked>
where
	F: FnMut(&str) -> hmerr::Result<Seen>,
{
	let mut dead = HashSet::new();
	let mut fallback = Linked::Dead;

	for id in ids {
		if dead.contains(&id) {
			continue;
		}

		match follow(&id, &mut dead, &mut see)? {
			Linked::Video => fallback = Linked::Video,
			Linked::Dead => {}
			song => return Ok(song),
		}
	}

	Ok(fallback)
}

fn follow<F>(listed: &str, dead: &mut HashSet<String>, see: &mut F) -> hmerr::Result<Linked>
where
	F: FnMut(&str) -> hmerr::Result<Seen>,
{
	let mut id = listed.to_string();

	loop {
		match see(&id)? {
			Seen::Song(info) => {
				let found = Found {
					url: verify::watch(&id),
					info,
				};

				return Ok(if id == listed {
					Linked::Song(found)
				} else {
					Linked::Replacement(found)
				});
			}
			Seen::Video => return Ok(Linked::Video),
			Seen::Gone { replacement } => {
				dead.insert(id);

				match replacement {
					Some(replacement) if !dead.contains(&replacement) => id = replacement,
					_ => return Ok(Linked::Dead),
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn song() -> Seen {
		Seen::Song(verify::Info {
			track: Some("Bad Apple!!".to_string()),
			artist: Some("Ateam".to_string()),
			duration: Some(239),
			media_type: Some("video".to_string()),
		})
	}

	fn gone(replacement: Option<&str>) -> Seen {
		Seen::Gone {
			replacement: replacement.map(str::to_string),
		}
	}

	fn resolved(ids: &[&str], see: impl Fn(&str) -> Seen) -> Linked {
		resolve_with(ids.iter().map(|id| (*id).to_string()).collect(), |id| {
			Ok(see(id))
		})
		.unwrap()
	}

	fn url(linked: &Linked) -> Option<&str> {
		match linked {
			Linked::Song(found) | Linked::Replacement(found) => Some(found.url.as_str()),
			Linked::Video | Linked::Dead => None,
		}
	}

	#[test]
	fn a_song_listed_after_a_video_is_kept() {
		let linked = resolved(&["video", "song"], |id| {
			if id == "song" { song() } else { Seen::Video }
		});

		assert!(matches!(linked, Linked::Song(_)));
		assert_eq!(url(&linked), Some(verify::watch("song").as_str()));
	}

	#[test]
	fn the_first_song_listed_wins_over_a_later_one() {
		let linked = resolved(&["first", "second"], |_| song());

		assert_eq!(url(&linked), Some(verify::watch("first").as_str()));
	}

	#[test]
	fn a_dead_link_redirecting_to_a_song_is_a_replacement() {
		let linked = resolved(&["dead"], |id| {
			if id == "dead" {
				gone(Some("alive"))
			} else {
				song()
			}
		});

		assert!(matches!(linked, Linked::Replacement(_)));
		assert_eq!(url(&linked), Some(verify::watch("alive").as_str()));
	}

	#[test]
	fn only_videos_ask_for_an_upgrade() {
		let linked = resolved(&["one", "two"], |_| Seen::Video);

		assert!(matches!(linked, Linked::Video));
	}

	#[test]
	fn a_video_beside_a_dead_link_still_asks_for_an_upgrade() {
		let linked = resolved(&["dead", "video"], |id| {
			if id == "dead" {
				gone(None)
			} else {
				Seen::Video
			}
		});

		assert!(matches!(linked, Linked::Video));
	}

	#[test]
	fn links_redirecting_into_each_other_end_dead() {
		let linked = resolved(&["a", "b"], |id| {
			gone(Some(if id == "a" { "b" } else { "a" }))
		});

		assert!(matches!(linked, Linked::Dead));
	}
}
