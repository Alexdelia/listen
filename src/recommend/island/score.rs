use serde::Serialize;

use crate::declaration::Source;

use super::{
	super::{attraction, known_artist},
	cohort::Member,
	index::Index,
};

pub(super) const MIN_BACKER: u32 = 5;

pub(super) const MIN_DISTINCT_BACKER: u32 = 3;

pub(super) const PER_ISLAND: usize = 200;

#[derive(Clone, Copy)]
pub(super) struct Tuning {
	pub damp: f32,
	pub allow_known_artist: bool,
	pub backing: Backing,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Backing {
	Head,
	Reach,
}

impl Backing {
	fn most_a_backer_counts(self) -> f64 {
		match self {
			Self::Head => 1.0,
			Self::Reach => f64::from(MIN_BACKER) / f64::from(MIN_DISTINCT_BACKER),
		}
	}
}

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
	tuning: Tuning,
) -> hmerr::Result<Vec<Vec<Candidate>>> {
	known_artist::declare(&index.db, tuning.allow_known_artist)?;
	enlist(index, cohort)?;

	let mut statement = index.db.prepare(&ranked())?;

	let mut row = statement.query(duckdb::params![
		tuning.backing.most_a_backer_counts(),
		MIN_BACKER,
		tuning.damp,
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
		c.liked_seed,
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
		count(*) filter (where attraction > 0) as backer,
		sum(least(sqrt(liked_seed), ?)) filter (where attraction > 0) as reach
	from vote
	group by 1, 2
),
eligible as (
	select b.recording_id, b.island, b.weight, b.backer, r.mbid, l.listener, l.plays
	from backing b
	join recording r using (recording_id)
	join recording_listener l using (recording_id)
	where b.reach >= ?
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
		"create or replace temp table cohort (island ubigint, user_id bigint, weight float, liked_seed uinteger);",
	)?;

	let mut appender = index.db.appender("cohort")?;
	for (island, cohort) in cohort.iter().enumerate() {
		for member in cohort {
			appender.append_row(duckdb::params![
				island as u64,
				member.user,
				member.weight,
				member.liked_seed
			])?;
		}
	}
	appender.flush()?;

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
				through: None,
				gap: Vec::new(),
				absorbed: 0,
				user: library.len() as u64,
				recording: 4,
				user_listen: 0,
			},
		}
	}

	fn cohort(member: u32) -> Vec<Vec<Member>> {
		cohort_liking(member, 1)
	}

	fn cohort_liking(member: u32, liked_seed: u32) -> Vec<Vec<Member>> {
		vec![
			(0..member)
				.map(|user| Member {
					user: i64::from(user),
					weight: 1.0,
					liked_seed,
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

	const UNKNOWN_ARTIST_ONLY: Tuning = Tuning {
		damp: POPULARITY_DAMP,
		allow_known_artist: false,
		backing: Backing::Head,
	};

	const REQUESTED: Tuning = Tuning {
		backing: Backing::Reach,
		..UNKNOWN_ARTIST_ONLY
	};

	fn served(index: &Index, member: u32) -> Vec<Candidate> {
		served_as(index, &cohort(member), UNKNOWN_ARTIST_ONLY)
	}

	fn served_as(index: &Index, cohort: &[Vec<Member>], tuning: Tuning) -> Vec<Candidate> {
		of(index, cohort, tuning)
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

	fn by_a_declared_artist() -> Index {
		let index = index(&every(&[(OTHER, 100)]), &uniform(MIN_BACKER));
		index
			.db
			.execute_batch(&format!(
				"insert into recording_artist values ({OTHER}, '{declared}');",
				declared = artist(SEED)
			))
			.unwrap();

		index
	}

	#[test]
	fn a_recording_by_a_declared_artist_stays_out() {
		assert!(served(&by_a_declared_artist(), MIN_BACKER).is_empty());
	}

	#[test]
	fn a_recording_by_a_declared_artist_comes_back_when_known_artists_are_allowed() {
		let candidate = served_as(
			&by_a_declared_artist(),
			&cohort(MIN_BACKER),
			Tuning {
				allow_known_artist: true,
				..UNKNOWN_ARTIST_ONLY
			},
		);

		assert_eq!(
			candidate
				.iter()
				.map(|candidate| candidate.mbid.to_string())
				.collect::<Vec<_>>(),
			vec![mbid(OTHER)]
		);
	}

	fn loved_by(member: u32) -> Index {
		let listen: Vec<(u32, u32, u32)> = (0..member).map(|user| (user, LOVED, 100)).collect();

		index(&listen, &uniform(member))
	}

	#[test]
	fn a_requested_island_lets_deep_backers_carry_what_too_few_heads_could_not() {
		let candidate = served_as(
			&loved_by(MIN_DISTINCT_BACKER),
			&cohort_liking(MIN_DISTINCT_BACKER, MIN_BACKER),
			REQUESTED,
		);

		assert_eq!(
			candidate
				.iter()
				.map(|candidate| (candidate.mbid.to_string(), candidate.backer))
				.collect::<Vec<_>>(),
			vec![(mbid(LOVED), MIN_DISTINCT_BACKER)]
		);
	}

	#[test]
	fn a_requested_island_never_lets_too_few_backers_carry_a_recording() {
		let few = MIN_DISTINCT_BACKER - 1;

		assert!(served_as(&loved_by(few), &cohort_liking(few, u32::MAX), REQUESTED).is_empty());
	}

	#[test]
	fn a_detected_island_counts_heads_however_many_liked_seeds_they_played() {
		assert!(
			served_as(
				&loved_by(MIN_DISTINCT_BACKER),
				&cohort_liking(MIN_DISTINCT_BACKER, MIN_BACKER),
				UNKNOWN_ARTIST_ONLY,
			)
			.is_empty()
		);
	}

	#[test]
	fn backers_who_played_a_single_liked_seed_each_count_as_one_head() {
		assert!(served(&loved_by(MIN_BACKER - 1), MIN_BACKER - 1).is_empty());
		assert!(!served(&loved_by(MIN_BACKER), MIN_BACKER).is_empty());
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
