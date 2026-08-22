use crate::declaration::Source;

use super::{attraction, cohort::Member, index::Index};

pub(super) const MIN_BACKER: u32 = 5;

const PER_ISLAND: usize = 200;

pub(super) struct Candidate {
	pub mbid: Source,
	pub score: f32,
	pub backer: u32,
	pub listener: u32,
	pub plays: u64,
}

pub(super) fn of(
	index: &Index,
	cohort: &[Vec<Member>],
	damp: f32,
) -> hmerr::Result<Vec<Vec<Candidate>>> {
	known_artist(index)?;
	enlist(index, cohort)?;

	let mut statement = index.db.prepare(&ranked())?;

	let mut row = statement.query(duckdb::params![
		MIN_BACKER,
		damp,
		i64::try_from(PER_ISLAND).unwrap_or(i64::MAX)
	])?;

	collected(&mut row, cohort.len())
}

fn ranked() -> String {
	format!(
		r"
with usual_library as (
	select median(recording)::float as recording from user_stat
),
vote as (
	select
		ul.recording_id,
		c.island,
		c.weight as cohort_weight,
		{weight}(ul.plays, s.center, s.low, s.high) as attraction,
		sqrt(greatest(s.recording, 1) / u.recording) as breadth
	from user_listen ul
	cross join usual_library u
	join user_stat s using (user_id)
	join cohort c on c.user_id = ul.user_id
),
backing as (
	select
		recording_id,
		island,
		sum(cohort_weight * attraction / breadth) as weight,
		count(*) filter (where attraction > 0) as backer
	from vote
	group by 1, 2
),
eligible as (
	select b.recording_id, b.island, b.weight, b.backer, r.mbid, l.listener, l.plays
	from backing b
	join recording r using (recording_id)
	join recording_listener l using (recording_id)
	where b.backer >= ?
		and b.weight > 0
		and not exists (select 1 from declared d where d.mbid::uuid = r.mbid)
		and not exists (
			select 1 from recording_artist ra
			semi join known_artist k on k.artist_mbid = ra.artist_mbid
			where ra.recording_id = b.recording_id
		)
),
scored as (
	select e.island, e.recording_id, e.mbid, e.backer, e.listener, e.plays,
		e.weight / pow(greatest(e.listener, 1), ?) as score
	from eligible e
),
per_artist as (
	select s.*,
		row_number() over (partition by ra.artist_mbid order by s.score desc, s.mbid) as rank
	from scored s
	join recording_artist ra using (recording_id)
),
best_of_artist as (
	select island, mbid, backer, listener, plays, score
	from per_artist
	group by all
	having max(rank) = 1
),
ranked as (
	select *, row_number() over (partition by island order by score desc, mbid) as position
	from best_of_artist
)
select island::bigint, mbid::varchar, score::float, backer::bigint, listener::bigint, plays::bigint
from ranked
where position <= ?
order by island, position
",
		weight = attraction::WEIGHT
	)
}

fn collected(row: &mut duckdb::Rows<'_>, island: usize) -> hmerr::Result<Vec<Vec<Candidate>>> {
	let mut candidate: Vec<Vec<Candidate>> = (0..island).map(|_| Vec::new()).collect();

	while let Some(row) = row.next()? {
		let island: i64 = row.get(0)?;
		let mbid: String = row.get(1)?;
		let score: f32 = row.get(2)?;
		let backer: i64 = row.get(3)?;
		let listener: i64 = row.get(4)?;
		let plays: i64 = row.get(5)?;

		let Ok(mbid) = mbid.parse() else {
			continue;
		};
		let Some(island) = usize::try_from(island)
			.ok()
			.and_then(|island| candidate.get_mut(island))
		else {
			continue;
		};

		island.push(Candidate {
			mbid,
			score,
			backer: u32::try_from(backer).unwrap_or(u32::MAX),
			listener: u32::try_from(listener).unwrap_or(u32::MAX),
			plays: u64::try_from(plays).unwrap_or(u64::MAX),
		});
	}

	Ok(candidate)
}

fn enlist(index: &Index, cohort: &[Vec<Member>]) -> hmerr::Result<()> {
	index.db.execute_batch(
		"create or replace temp table cohort (island ubigint, user_id bigint, weight float);",
	)?;

	let mut appender = index.db.appender("cohort")?;
	for (island, cohort) in cohort.iter().enumerate() {
		for member in cohort {
			appender.append_row(duckdb::params![island as u64, member.user, member.weight])?;
		}
	}
	appender.flush()?;

	Ok(())
}

fn known_artist(index: &Index) -> hmerr::Result<()> {
	index.db.execute_batch(
		r"
create or replace temp table known_artist as
with seed_artist as (
	select distinct ra.artist_mbid
	from declared d
	join recording r on r.mbid = d.mbid::uuid
	join recording_artist ra using (recording_id)
)
select artist_mbid from seed_artist
union
select al.related_mbid
from artist_link al
semi join seed_artist s on s.artist_mbid = al.artist_mbid;
",
	)?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::args::POPULARITY_DAMP;

	use super::{super::index::Meta, *};

	const SEED: u32 = 0;
	const LOVED: u32 = 1;
	const BRUSHED: u32 = 2;
	const OTHER: u32 = 3;

	const CENTER_PLAY: f32 = 10.0;
	const HIGH_PLAY: f32 = 100.0;
	const LIBRARY: u32 = 100;

	fn mbid(recording: u32) -> String {
		format!("00000000-0000-0000-0000-0000000000{recording:02x}")
	}

	fn artist(recording: u32) -> String {
		format!("11111111-0000-0000-0000-0000000000{recording:02x}")
	}

	fn index(listen: &[(u32, u32, u32)], library: &[(u32, u32)]) -> Index {
		let db = duckdb::Connection::open_in_memory().unwrap();
		attraction::declare(&db).unwrap();

		db.execute_batch(&format!(
			r"
create table recording (recording_id uinteger, mbid uuid);
create table recording_artist (recording_id uinteger, artist_mbid uuid);
create table artist_link (artist_mbid uuid, related_mbid uuid);
create table user_listen (user_id uinteger, recording_id uinteger, plays usmallint);
create table user_stat (user_id uinteger, center float, low float, high float, recording uinteger);
create table declared (mbid varchar, q utinyint);
insert into recording values
	({SEED}, '{seed}'), ({LOVED}, '{loved}'),
	({BRUSHED}, '{brushed}'), ({OTHER}, '{other}');
insert into recording_artist values
	({SEED}, '{seed_artist}'), ({LOVED}, '{loved_artist}'),
	({BRUSHED}, '{brushed_artist}'), ({OTHER}, '{other_artist}');
insert into declared values ('{seed}', 4);
insert into user_stat values {library};
insert into user_listen values {listen};
create table recording_listener as
	select recording_id, count(*)::uinteger as listener, sum(plays)::ubigint as plays
	from user_listen group by 1;
",
			seed = mbid(SEED),
			loved = mbid(LOVED),
			brushed = mbid(BRUSHED),
			other = mbid(OTHER),
			seed_artist = artist(SEED),
			loved_artist = artist(LOVED),
			brushed_artist = artist(BRUSHED),
			other_artist = artist(OTHER),
			library = library
				.iter()
				.map(|(user, recording)| format!(
					"({user}, {center}, 0, {high}, {recording})",
					center = CENTER_PLAY.ln(),
					high = HIGH_PLAY.ln()
				))
				.collect::<Vec<_>>()
				.join(","),
			listen = listen
				.iter()
				.map(|(user, recording, plays)| format!("({user}, {recording}, {plays})"))
				.collect::<Vec<_>>()
				.join(","),
		))
		.unwrap();

		Index {
			db,
			meta: Meta {
				built: String::new(),
				dump: String::new(),
				own: None,
				reached: None,
				gap: Vec::new(),
				absorbed: 0,
				user: library.len() as u64,
				recording: 4,
				user_listen: 0,
			},
		}
	}

	fn cohort(member: u32) -> Vec<Vec<Member>> {
		vec![
			(0..member)
				.map(|user| Member {
					user: i64::from(user),
					weight: 1.0,
				})
				.collect(),
		]
	}

	fn uniform(member: u32) -> Vec<(u32, u32)> {
		(0..member).map(|user| (user, LIBRARY)).collect()
	}

	fn every(listen: &[(u32, u32)]) -> Vec<(u32, u32, u32)> {
		(0..MIN_BACKER)
			.flat_map(|user| {
				listen
					.iter()
					.map(move |(recording, plays)| (user, *recording, *plays))
			})
			.collect()
	}

	fn served(index: &Index, member: u32) -> Vec<Candidate> {
		of(index, &cohort(member), POPULARITY_DAMP)
			.unwrap()
			.into_iter()
			.next()
			.unwrap_or_default()
	}

	fn candidate(listen: &[(u32, u32)]) -> Vec<Candidate> {
		served(&index(&every(listen), &uniform(MIN_BACKER)), MIN_BACKER)
	}

	fn score(candidate: &[Candidate], recording: u32) -> f32 {
		candidate
			.iter()
			.find(|candidate| candidate.mbid.to_string() == mbid(recording))
			.unwrap_or_else(|| panic!("no candidate for recording {recording}"))
			.score
	}

	#[test]
	fn a_recording_the_whole_cohort_repeats_is_a_candidate() {
		let candidate = candidate(&[(LOVED, 100)]);

		assert_eq!(
			candidate
				.iter()
				.map(|candidate| candidate.mbid.to_string())
				.collect::<Vec<_>>(),
			vec![mbid(LOVED)]
		);
		assert_eq!(candidate.first().map(|candidate| candidate.backer), Some(5));
	}

	#[test]
	fn a_recording_the_whole_cohort_played_once_and_dropped_is_no_candidate() {
		assert!(candidate(&[(BRUSHED, 1)]).is_empty());
	}

	#[test]
	fn a_recording_most_of_the_cohort_tried_and_dropped_is_no_candidate() {
		let lover = 2;
		let listen: Vec<(u32, u32, u32)> = (0..MIN_BACKER)
			.map(|user| (user, LOVED, if user < lover { 100 } else { 1 }))
			.collect();

		assert!(served(&index(&listen, &uniform(MIN_BACKER)), MIN_BACKER).is_empty());
	}

	#[test]
	fn a_declared_recording_never_comes_back_as_a_candidate() {
		assert!(
			!candidate(&[(SEED, 100), (LOVED, 100)])
				.iter()
				.any(|candidate| candidate.mbid.to_string() == mbid(SEED))
		);
	}

	#[test]
	fn a_recording_sharing_an_artist_with_a_better_one_stays_out() {
		let index = index(&every(&[(LOVED, 100), (OTHER, 100)]), &uniform(MIN_BACKER));
		index
			.db
			.execute_batch(&format!(
				"insert into recording_artist values ({OTHER}, '{shared}');",
				shared = artist(LOVED)
			))
			.unwrap();

		let candidate = served(&index, MIN_BACKER);

		assert_eq!(
			candidate
				.iter()
				.map(|candidate| candidate.mbid.to_string())
				.collect::<Vec<_>>(),
			vec![mbid(LOVED)]
		);
	}

	#[test]
	fn a_recording_more_of_the_pool_plays_scores_below_an_equally_loved_rarity() {
		let crowd = 20;
		let known: Vec<(u32, u32, u32)> = (MIN_BACKER..MIN_BACKER + crowd)
			.map(|user| (user, LOVED, 100))
			.collect();
		let listen = [every(&[(LOVED, 100), (OTHER, 100)]), known].concat();

		let candidate = served(&index(&listen, &uniform(MIN_BACKER)), MIN_BACKER);

		assert!(score(&candidate, OTHER) > score(&candidate, LOVED));
	}

	#[test]
	fn a_listener_repeating_a_recording_forever_never_makes_it_popular() {
		let obsessed: Vec<(u32, u32, u32)> = (0..MIN_BACKER)
			.map(|user| (user, LOVED, u32::from(u16::MAX)))
			.collect();
		let listen = [obsessed, every(&[(OTHER, 100)])].concat();

		let candidate = served(&index(&listen, &uniform(MIN_BACKER)), MIN_BACKER);

		assert_eq!(
			candidate
				.iter()
				.map(|candidate| candidate.listener)
				.collect::<Vec<_>>(),
			vec![MIN_BACKER, MIN_BACKER]
		);
	}

	#[test]
	fn a_wider_library_carries_a_lighter_vote() {
		let member = MIN_BACKER * 2;
		let narrow: Vec<(u32, u32, u32)> = (0..MIN_BACKER).map(|user| (user, LOVED, 100)).collect();
		let wide: Vec<(u32, u32, u32)> = (MIN_BACKER..member)
			.map(|user| (user, OTHER, 100))
			.collect();

		let library: Vec<(u32, u32)> = (0..member)
			.map(|user| {
				(
					user,
					if user < MIN_BACKER {
						LIBRARY
					} else {
						LIBRARY * 100
					},
				)
			})
			.collect();

		let candidate = served(&index(&[narrow, wide].concat(), &library), member);

		assert!(score(&candidate, LOVED) > score(&candidate, OTHER));
	}
}
