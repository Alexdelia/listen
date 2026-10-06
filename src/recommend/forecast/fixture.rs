use std::{collections::BTreeSet, ops::Range};

use crate::declaration::{Q, Source};

use super::{super::known_artist, predict, preference};

pub(super) const LOVED: u16 = 50;
pub(super) const ONCE: u16 = 1;

const CENTER_PLAY: f32 = 10.0;
const ARTIST_BASE: u128 = 1 << 64;

pub(super) fn mbid(recording: u32) -> Source {
	Source::from_u128(u128::from(recording) + 1)
}

fn artist(recording: u32, shared: &[(u32, u32)]) -> Source {
	let artist = shared
		.iter()
		.find(|(of, _)| *of == recording)
		.map_or(recording, |(_, artist)| *artist);

	Source::from_u128(ARTIST_BASE + u128::from(artist))
}

pub(super) fn fans(user: Range<u32>, recording: u32, plays: u16) -> Vec<(u32, u32, u16)> {
	user.map(|user| (user, recording, plays)).collect()
}

pub(super) fn db(
	listen: &[(u32, u32, u16)],
	declared: &[(u32, Q)],
	shared_artist: &[(u32, u32)],
) -> duckdb::Connection {
	let db = duckdb::Connection::open_in_memory().unwrap();

	let recording: BTreeSet<u32> = listen
		.iter()
		.map(|(_, recording, _)| *recording)
		.chain(declared.iter().map(|(recording, _)| *recording))
		.collect();
	let user: BTreeSet<u32> = listen.iter().map(|(user, _, _)| *user).collect();

	let rows = |row: Vec<String>| row.join(", ");

	db.execute_batch(&format!(
		r"
create table recording (recording_id uinteger, mbid uuid);
create table recording_artist (recording_id uinteger, artist_mbid uuid);
create table artist_link (artist_mbid uuid, related_mbid uuid);
create table user_listen (user_id uinteger, recording_id uinteger, plays usmallint);
create table user_stat (user_id uinteger, center float, low float, high float, recording uinteger);
insert into recording values {recording_row};
insert into recording_artist values {artist_row};
insert into user_listen values {listen_row};
insert into user_stat values {stat_row};
",
		recording_row = rows(
			recording
				.iter()
				.map(|recording| format!("({recording}, '{}')", mbid(*recording)))
				.collect()
		),
		artist_row = rows(
			recording
				.iter()
				.map(|recording| format!("({recording}, '{}')", artist(*recording, shared_artist)))
				.collect()
		),
		listen_row = rows(
			listen
				.iter()
				.map(|(user, recording, plays)| format!("({user}, {recording}, {plays})"))
				.collect()
		),
		stat_row = rows(
			user.iter()
				.map(|user| format!(
					"({user}, {center}, 0, {center}, 100)",
					center = CENTER_PLAY.ln()
				))
				.collect()
		),
	))
	.unwrap();

	db.execute_batch("create or replace temp table declared (mbid varchar, q utinyint);")
		.unwrap();
	for (recording, q) in declared {
		db.execute(
			"insert into declared values (?, ?)",
			duckdb::params![mbid(*recording).to_string(), q],
		)
		.unwrap();
	}

	preference::declare(&db).unwrap();
	known_artist::declare(&db, false).unwrap();
	predict::prepare(&db).unwrap();

	db
}
