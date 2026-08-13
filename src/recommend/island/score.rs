use crate::declaration::Source;

use super::{cohort::Member, index::Index};

pub(super) const POPULARITY_DAMP: f32 = 0.6;
pub(super) const MIN_BACKER: u32 = 5;

const PER_ISLAND: usize = 200;

pub(super) struct Candidate {
	pub mbid: Source,
	pub score: f32,
	pub backer: u32,
	pub plays: u32,
}

pub(super) fn of(
	index: &Index,
	cohort: &[Vec<Member>],
	damp: f32,
) -> hmerr::Result<Vec<Vec<Candidate>>> {
	known_artist(index)?;
	enlist(index, cohort)?;

	let mut candidate: Vec<Vec<Candidate>> = (0..cohort.len()).map(|_| Vec::new()).collect();

	let mut statement = index.db.prepare(
		r"
with backing as (
	select ul.recording_id, c.island, sum(c.weight) as weight, count(*) as backer
	from user_listen ul
	join cohort c on c.user_id = ul.user_id
	group by 1, 2
),
eligible as (
	select b.recording_id, b.island, b.weight, b.backer, r.mbid, r.global_plays
	from backing b
	join recording r using (recording_id)
	where b.backer >= ?
		and not exists (select 1 from declared d where d.mbid::uuid = r.mbid)
		and not exists (
			select 1 from recording_artist ra
			semi join known_artist k on k.artist_mbid = ra.artist_mbid
			where ra.recording_id = b.recording_id
		)
),
credit as (
	select ra.recording_id,
		string_agg(ra.artist_mbid::varchar, ',' order by ra.artist_mbid) as credit
	from recording_artist ra
	semi join eligible e on e.recording_id = ra.recording_id
	group by 1
),
scored as (
	select e.island, e.mbid, e.backer, e.global_plays, c.credit,
		e.weight / pow(greatest(e.global_plays, 1), ?) as score
	from eligible e
	join credit c using (recording_id)
),
best_of_credit as (
	select *, row_number() over (partition by credit order by score desc, mbid) as rank
	from scored
),
ranked as (
	select *, row_number() over (partition by island order by score desc, mbid) as position
	from best_of_credit
	where rank = 1
)
select island::bigint, mbid::varchar, score::float, backer::bigint, global_plays::bigint
from ranked
where position <= ?
order by island, position
",
	)?;

	let mut row = statement.query(duckdb::params![
		MIN_BACKER,
		damp,
		i64::try_from(PER_ISLAND).unwrap_or(i64::MAX)
	])?;

	while let Some(row) = row.next()? {
		let island: i64 = row.get(0)?;
		let mbid: String = row.get(1)?;
		let score: f32 = row.get(2)?;
		let backer: i64 = row.get(3)?;
		let plays: i64 = row.get(4)?;

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
			plays: u32::try_from(plays).unwrap_or(u32::MAX),
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
