use std::io::{BufRead, BufReader, Read};

use ansi::abbrev::{B, D, F, R};
use hmerr::ge;
use serde::Deserialize;

use super::{
	super::{
		board::{Board, Planned},
		progress::Measure,
	},
	listen, rsync, stamp,
};

const MODULE: &str = "listenbrainz/incremental";
const SUFFIX: &str = "-incremental";
const ARCHIVE: &str = "listenbrainz-listens-dump-";
const EXT: &str = ".tar.zst";
const OVER: &str = "https://data.metabrainz.org/pub/musicbrainz";
const LISTENS: &str = ".listens";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
	Name,
}

impl Planned for Stage {
	type Cost = u64;

	fn title(self) -> &'static str {
		"name"
	}

	fn measure(self, size: &u64) -> Measure {
		Measure::Byte(*size)
	}
}

const READ: usize = 64 * 1024;
const AT_MOST: usize = 1;

#[derive(Deserialize)]
struct Listener {
	user_id: u32,
	user_name: String,
}

struct Published {
	url: String,
	size: u64,
	reach: u64,
}

pub(crate) struct Search {
	pub id: Option<u32>,
	pub reach: Option<u64>,
}

pub(super) fn named(username: &str, past: Option<u64>) -> hmerr::Result<Search> {
	let mut search = Search {
		id: None,
		reach: past,
	};

	for published in latest()? {
		if !unread(&published, past) {
			continue;
		}

		search.reach = Some(published.reach);

		if let Some(id) = read(&published, username)? {
			search.id = Some(id);
			return Ok(search);
		}
	}

	Ok(search)
}

fn unread(published: &Published, past: Option<u64>) -> bool {
	past.is_none_or(|reach| published.reach > reach)
}

fn latest() -> hmerr::Result<Vec<Published>> {
	let url = format!("{host}/{MODULE}/", host = rsync::HOST);

	let mut found: Vec<Published> = rsync::beneath(&url, &format!("{ARCHIVE}*{SUFFIX}{EXT}"))?
		.iter()
		.filter_map(published_of)
		.collect();

	found.sort_by_key(|published| std::cmp::Reverse(published.reach));
	found.truncate(AT_MOST);

	Ok(found)
}

fn published_of(entry: &rsync::Entry) -> Option<Published> {
	let (dir, archive) = entry.name.split_once('/')?;

	Some(Published {
		reach: stamp::published(dir, listen::PREFIX, SUFFIX)?.reach,
		url: format!("{OVER}/{MODULE}/{dir}/{archive}"),
		size: entry.size,
	})
}

fn read(published: &Published, username: &str) -> hmerr::Result<Option<u32>> {
	let board = Board::of(&[Stage::Name], &published.size)?;

	board.run(Stage::Name, |bar| {
		let body = ureq::get(&published.url)
			.call()
			.map_err(|e| unreachable_dump(&published.url, &e.to_string()))?
			.into_body()
			.into_reader();

		listed(bar.wrap_read(body), username)
	})
}

fn listed(read: impl Read, username: &str) -> hmerr::Result<Option<u32>> {
	let mut archive = tar::Archive::new(zstd::Decoder::new(read)?);

	for entry in archive.entries()? {
		let entry = entry?;

		if !entry.path()?.to_string_lossy().ends_with(LISTENS) {
			continue;
		}

		if let Some(id) = lined(entry, username)? {
			return Ok(Some(id));
		}
	}

	Ok(None)
}

fn lined(read: impl Read, username: &str) -> hmerr::Result<Option<u32>> {
	let needle = needle(username)?;
	let mut reader = BufReader::with_capacity(READ, read);
	let mut line = Vec::new();

	loop {
		line.clear();

		if reader.read_until(b'\n', &mut line)? == 0 {
			return Ok(None);
		}

		if !String::from_utf8_lossy(&line).contains(&needle) {
			continue;
		}

		if let Ok(listener) = serde_json::from_slice::<Listener>(&line)
			&& listener.user_name == username
		{
			return Ok(Some(listener.user_id));
		}
	}
}

fn needle(username: &str) -> hmerr::Result<String> {
	Ok(format!(
		"\"user_name\":{quoted}",
		quoted = serde_json::to_string(username)?
	))
}

fn unreachable_dump(url: &str, reason: &str) -> Box<dyn std::error::Error> {
	ge!(
		format!("{R}failed to fetch {B}{url}{D}\n{reason}"),
		h: format!("{F}the listens dump is what names a listener, the run goes on without it{D}")
	)
	.into()
}

#[cfg(test)]
mod tests {
	use super::*;

	const LINE: &str = r#"{"user_id":35598,"user_name":"Alexdelia","timestamp":1787362067,"track_metadata":{"track_name":"Fairy Dance","artist_name":"UNDEAD CORPORATION"}}"#;
	const OTHER: &str = r#"{"user_id":123,"user_name":"someone","timestamp":1787362067,"track_metadata":{"track_name":"SWIM","artist_name":"BTS"}}"#;

	fn lines(line: &[&str]) -> String {
		format!("{}\n", line.join("\n"))
	}

	#[test]
	fn a_listen_dumped_under_a_name_gives_the_number_it_was_listened_under() {
		assert_eq!(
			lined(lines(&[OTHER, LINE]).as_bytes(), "Alexdelia").unwrap_or_default(),
			Some(35598)
		);
	}

	#[test]
	fn a_name_nobody_listened_under_is_named_by_nothing() {
		assert_eq!(
			lined(lines(&[OTHER, LINE]).as_bytes(), "nobody").unwrap_or_default(),
			None
		);
	}

	#[test]
	fn a_name_another_name_merely_starts_with_is_not_a_match() {
		assert_eq!(
			lined(lines(&[LINE]).as_bytes(), "Alexdel").unwrap_or_default(),
			None
		);
	}

	#[test]
	fn a_line_that_is_no_listen_is_read_past() {
		assert_eq!(
			lined(
				lines(&["not json at all", OTHER, LINE]).as_bytes(),
				"Alexdelia"
			)
			.unwrap_or_default(),
			Some(35598)
		);
	}

	#[test]
	fn a_name_carrying_what_json_escapes_is_looked_for_as_it_is_written() {
		assert_eq!(
			needle(r#"quo"te"#).unwrap_or_default(),
			r#""user_name":"quo\"te""#
		);
	}

	#[test]
	fn a_dump_published_past_the_one_already_read_is_read_in_turn() {
		assert!(unread(
			&Published {
				url: String::new(),
				size: 0,
				reach: 20_260_823_000_003,
			},
			Some(20_260_822_000_003)
		));
	}

	#[test]
	fn a_dump_no_newer_than_the_one_already_read_is_left_alone() {
		assert!(!unread(
			&Published {
				url: String::new(),
				size: 0,
				reach: 20_260_822_000_003,
			},
			Some(20_260_822_000_003)
		));
	}

	#[test]
	fn every_dump_is_worth_reading_when_none_was_read_before() {
		assert!(unread(
			&Published {
				url: String::new(),
				size: 0,
				reach: 20_260_822_000_003,
			},
			None
		));
	}

	#[test]
	fn only_the_listens_of_the_dump_are_read_through() {
		assert_eq!(
			published_of(&rsync::Entry {
				name: "listenbrainz-dump-2637-20260823-000003-incremental/\
					listenbrainz-listens-dump-2637-20260823-000003-incremental.tar.zst"
					.to_string(),
				size: 221_113_842,
			})
			.map(|published| published.reach),
			Some(20_260_823_000_003)
		);
	}
}
