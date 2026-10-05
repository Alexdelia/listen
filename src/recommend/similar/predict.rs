use crate::declaration::{Q, Q_MAX, Source, value};

use super::preference::PREFERENCE;

pub(super) const MIN_SUPPORT: u32 = 5;

pub(super) const NEUTRAL_PRIOR: f32 = 10.0;

#[derive(Clone, Copy)]
pub(super) struct Scored {
	pub recording_id: u32,
	pub mbid: Source,
	pub raw: f32,
	pub support: u32,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Point {
	pub mbid: Source,
	pub q: Q,
	pub raw: f32,
}

pub(super) fn weight_list() -> String {
	let weight: Vec<String> = (0..=Q_MAX).map(|q| value::weight(q).to_string()).collect();

	format!("[{}]", weight.join(", "))
}

pub(super) fn prepare(db: &duckdb::Connection) -> hmerr::Result<()> {
	db.execute_batch(&format!(
		r"
create or replace temp table similar_declared as
	select r.recording_id, d.q
	from declared d join recording r on r.mbid = d.mbid::uuid;
create or replace temp table similar_declared_listen as
	select ul.user_id, sd.recording_id, sd.q, {PREFERENCE}(ul.plays, s.center) as preference
	from user_listen ul
	join user_stat s using (user_id)
	join similar_declared sd using (recording_id);
create or replace temp table similar_spread as
	select recording_id, sqrt(sum(preference * preference)) as spread
	from similar_declared_listen
	group by 1;
create or replace temp table similar_listener as
	select l.user_id,
		sum(l.preference * {weight}[l.q + 1] / p.spread) as taste,
		sum(l.preference / p.spread) as taste_scale
	from similar_declared_listen l
	join similar_spread p using (recording_id)
	where l.preference > 0
	group by 1;
",
		weight = weight_list()
	))?;

	Ok(())
}

pub(super) fn candidates(db: &duckdb::Connection) -> hmerr::Result<Vec<Scored>> {
	let neutral = f32::from(value::NEUTRAL);
	let mut statement = db.prepare(&format!(
		r"
with listen as (
	select ul.recording_id, {PREFERENCE}(ul.plays, s.center) as preference, l.taste, l.taste_scale
	from user_listen ul
	join user_stat s using (user_id)
	join similar_listener l using (user_id)
),
scored as (
	select recording_id,
		sum(preference * taste) / (sum(abs(preference) * taste_scale) + {NEUTRAL_PRIOR}) as predicted,
		count(*) filter (where preference > 0) as support
	from listen
	group by 1
	having count(*) filter (where preference > 0) >= ?
),
eligible as (
	select s.recording_id, s.predicted, s.support, r.mbid
	from scored s
	join recording r using (recording_id)
	where s.predicted is not null
		and not isnan(s.predicted)
		and not exists (select 1 from similar_declared d where d.recording_id = s.recording_id)
		and not exists (
			select 1 from recording_artist ra
			semi join known_artist k on k.artist_mbid = ra.artist_mbid
			where ra.recording_id = s.recording_id
		)
),
per_artist as (
	select e.*,
		row_number() over (partition by ra.artist_mbid order by e.predicted desc, e.mbid) as rank
	from eligible e
	join recording_artist ra using (recording_id)
),
best as (
	select recording_id, mbid, predicted, support
	from per_artist
	group by all
	having max(rank) = 1
)
select recording_id::bigint, mbid::varchar, ({neutral} + {neutral} * predicted)::float, support::bigint
from best
"
	))?;

	let mut row = statement.query(duckdb::params![MIN_SUPPORT])?;
	let mut scored = Vec::new();

	while let Some(row) = row.next()? {
		let recording_id: i64 = row.get(0)?;
		let mbid: String = row.get(1)?;
		let raw: f32 = row.get(2)?;
		let support: i64 = row.get(3)?;

		let (Ok(recording_id), Ok(mbid)) = (u32::try_from(recording_id), mbid.parse()) else {
			continue;
		};

		scored.push(Scored {
			recording_id,
			mbid,
			raw,
			support: u32::try_from(support).unwrap_or(u32::MAX),
		});
	}

	Ok(scored)
}

pub(super) fn rated(db: &duckdb::Connection, rated: &[(Source, Q)]) -> hmerr::Result<Vec<Point>> {
	db.execute_batch("create or replace temp table similar_rated (mbid varchar, q utinyint);")?;
	{
		let mut appender = db.appender("similar_rated")?;
		for (mbid, q) in rated {
			appender.append_row(duckdb::params![mbid.to_string(), q])?;
		}
		appender.flush()?;
	}

	let neutral = f32::from(value::NEUTRAL);
	let mut statement = db.prepare(&format!(
		r"
with rated as (
	select r.recording_id, sr.mbid, sr.q
	from similar_rated sr join recording r on r.mbid = sr.mbid::uuid
),
listen as (
	select rt.recording_id, rt.mbid, rt.q, {PREFERENCE}(ul.plays, s.center) as preference,
		l.taste, l.taste_scale, p.spread
	from user_listen ul
	join rated rt using (recording_id)
	join user_stat s using (user_id)
	join similar_listener l using (user_id)
	join similar_spread p using (recording_id)
),
left_out as (
	select recording_id, mbid, q, preference,
		taste - greatest(preference, 0) * {weight}[q + 1] / spread as taste,
		taste_scale - greatest(preference, 0) / spread as taste_scale
	from listen
),
judged as (
	select any_value(mbid) as mbid, any_value(q) as q,
		sum(preference * taste) / (sum(abs(preference) * taste_scale) + {NEUTRAL_PRIOR}) as predicted
	from left_out
	group by recording_id
)
select mbid, q::utinyint, ({neutral} + {neutral} * predicted)::float
from judged
where predicted is not null and not isnan(predicted)
",
		weight = weight_list()
	))?;

	let mut row = statement.query([])?;
	let mut point = Vec::new();

	while let Some(row) = row.next()? {
		let mbid: String = row.get(0)?;
		let Ok(mbid) = mbid.parse() else {
			continue;
		};

		point.push(Point {
			mbid,
			q: row.get(1)?,
			raw: row.get(2)?,
		});
	}

	Ok(point)
}

#[cfg(test)]
mod tests {
	use super::{
		super::fixture::{self, LOVED, ONCE, fans, mbid},
		*,
	};

	const LIKED_SEED: u32 = 0;
	const AVOIDED_SEED: u32 = 1;
	const NEAR_LIKED: u32 = 2;
	const NEAR_AVOIDED: u32 = 3;
	const OTHER: u32 = 4;
	const SILENT_SEED: u32 = 5;

	fn raw(scored: &[Scored], recording: u32) -> Option<f32> {
		scored
			.iter()
			.find(|scored| scored.mbid == mbid(recording))
			.map(|scored| scored.raw)
	}

	fn shrunk(weight: f32, loving_fan: u16) -> f32 {
		let mass = f32::from(loving_fan).sqrt();

		weight.mul_add(50.0 * mass / (mass + NEUTRAL_PRIOR), 50.0)
	}

	fn served(
		listen: &[Vec<(u32, u32, u16)>],
		declared: &[(u32, Q)],
		shared: &[(u32, u32)],
	) -> Vec<Scored> {
		candidates(&fixture::db(&listen.concat(), declared, shared)).unwrap()
	}

	#[test]
	fn a_q0_neighbour_pulls_a_candidate_under_one_an_equal_q3_neighbour_lifts() {
		let scored = served(
			&[
				fans(0..5, LIKED_SEED, LOVED),
				fans(0..5, NEAR_LIKED, LOVED),
				fans(5..10, AVOIDED_SEED, LOVED),
				fans(5..10, NEAR_AVOIDED, LOVED),
			],
			&[(LIKED_SEED, 3), (AVOIDED_SEED, 0)],
			&[],
		);

		let liked = raw(&scored, NEAR_LIKED).unwrap_or_default();
		let avoided = raw(&scored, NEAR_AVOIDED).unwrap_or(100.0);

		assert!(
			(liked - shrunk(value::weight(3), 5)).abs() < 1e-3,
			"{liked}"
		);
		assert!(
			(avoided - shrunk(value::weight(0), 5)).abs() < 1e-3,
			"{avoided}"
		);
	}

	#[test]
	fn a_candidate_few_listeners_back_sits_closer_to_neutral_than_one_many_back() {
		let scored = served(
			&[
				fans(0..40, LIKED_SEED, LOVED),
				fans(0..5, NEAR_LIKED, LOVED),
				fans(0..40, OTHER, LOVED),
			],
			&[(LIKED_SEED, 3)],
			&[],
		);

		let few = raw(&scored, NEAR_LIKED).unwrap_or(100.0);
		let many = raw(&scored, OTHER).unwrap_or_default();

		assert!(50.0 < few && few < many, "{few} {many}");
	}

	#[test]
	fn listeners_who_tried_a_candidate_once_pull_it_down() {
		let scored = served(
			&[
				fans(0..8, LIKED_SEED, LOVED),
				fans(0..5, NEAR_LIKED, LOVED),
				fans(5..8, NEAR_LIKED, ONCE),
				fans(0..5, OTHER, LOVED),
			],
			&[(LIKED_SEED, 3)],
			&[],
		);

		let tried = raw(&scored, NEAR_LIKED).unwrap_or(100.0);
		let kept = raw(&scored, OTHER).unwrap_or_default();

		assert!(tried < kept, "{tried} {kept}");
	}

	#[test]
	fn a_candidate_short_of_min_support_is_left_out() {
		let short = MIN_SUPPORT - 1;
		let scored = served(
			&[
				fans(0..5, LIKED_SEED, LOVED),
				fans(0..short, NEAR_LIKED, LOVED),
			],
			&[(LIKED_SEED, 3)],
			&[],
		);

		assert_eq!(raw(&scored, NEAR_LIKED), None);
	}

	#[test]
	fn a_declared_recording_is_never_a_candidate() {
		let scored = served(
			&[fans(0..5, LIKED_SEED, LOVED), fans(0..5, NEAR_LIKED, LOVED)],
			&[(LIKED_SEED, 3)],
			&[],
		);

		assert_eq!(raw(&scored, LIKED_SEED), None);
	}

	#[test]
	fn a_candidate_by_a_declared_artist_is_left_out() {
		let scored = served(
			&[fans(0..5, LIKED_SEED, LOVED), fans(0..5, NEAR_LIKED, LOVED)],
			&[(LIKED_SEED, 3)],
			&[(NEAR_LIKED, LIKED_SEED)],
		);

		assert_eq!(raw(&scored, NEAR_LIKED), None);
	}

	#[test]
	fn only_the_best_recording_of_an_artist_is_kept() {
		let scored = served(
			&[
				fans(0..10, LIKED_SEED, LOVED),
				fans(5..10, AVOIDED_SEED, LOVED),
				fans(0..5, NEAR_LIKED, LOVED),
				fans(0..10, OTHER, LOVED),
			],
			&[(LIKED_SEED, 3), (AVOIDED_SEED, 0)],
			&[(OTHER, NEAR_LIKED)],
		);

		assert!(raw(&scored, NEAR_LIKED).is_some());
		assert_eq!(raw(&scored, OTHER), None);
	}

	#[test]
	fn a_candidate_only_strangers_love_is_never_scored() {
		let scored = served(
			&[fans(0..5, LIKED_SEED, LOVED), fans(5..10, OTHER, LOVED)],
			&[(LIKED_SEED, 3)],
			&[],
		);

		assert_eq!(raw(&scored, OTHER), None);
	}

	#[test]
	fn a_declaration_nobody_in_the_index_plays_changes_nothing() {
		let listen = [fans(0..5, LIKED_SEED, LOVED), fans(0..5, NEAR_LIKED, LOVED)];
		let without = served(&listen, &[(LIKED_SEED, 3)], &[]);
		let with = served(&listen, &[(LIKED_SEED, 3), (SILENT_SEED, 0)], &[]);

		assert_eq!(raw(&without, NEAR_LIKED), raw(&with, NEAR_LIKED));
	}

	#[test]
	fn a_rated_pick_is_judged_without_its_own_declaration() {
		let db = fixture::db(
			&[fans(0..5, LIKED_SEED, LOVED), fans(0..5, NEAR_LIKED, LOVED)].concat(),
			&[(LIKED_SEED, 3), (NEAR_LIKED, 0)],
			&[],
		);

		let point = rated(&db, &[(mbid(NEAR_LIKED), 0)]).unwrap();

		assert_eq!(point.len(), 1);
		let judged = point.first().copied().unwrap_or_default();
		assert_eq!(judged.mbid, mbid(NEAR_LIKED));
		assert_eq!(judged.q, 0);
		assert!(
			(judged.raw - shrunk(value::weight(3), 5)).abs() < 1e-3,
			"{}",
			judged.raw
		);
	}

	#[test]
	fn a_rated_pick_nobody_plays_is_left_out_of_the_calibration() {
		let db = fixture::db(
			&fans(0..5, LIKED_SEED, LOVED),
			&[(LIKED_SEED, 3), (SILENT_SEED, 2)],
			&[],
		);

		assert!(rated(&db, &[(mbid(SILENT_SEED), 2)]).unwrap().is_empty());
	}
}
