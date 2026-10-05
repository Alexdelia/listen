use std::collections::HashMap;

use crate::declaration::{Q, Source, value};

use super::{predict::Scored, preference::PREFERENCE};

pub(super) const NEAR: usize = 3;

#[derive(Default)]
pub(super) struct Profile(HashMap<u32, f32>);

impl Profile {
	pub(super) fn of_strength(strength: impl IntoIterator<Item = (u32, f32)>) -> Self {
		Self(strength.into_iter().collect())
	}

	pub(super) fn cosine(&self, other: &Self) -> f32 {
		let dot: f32 = self
			.0
			.iter()
			.filter_map(|(seed, strength)| other.0.get(seed).map(|other| strength * other))
			.sum();
		let norm = self.norm() * other.norm();

		if norm <= f32::EPSILON {
			return 0.0;
		}

		dot / norm
	}

	fn norm(&self) -> f32 {
		self.0
			.values()
			.map(|strength| strength * strength)
			.sum::<f32>()
			.sqrt()
	}
}

pub(super) struct Near {
	pub mbid: Source,
	pub q: Q,
}

pub(super) struct Shape {
	pub profile: Profile,
	pub near: Vec<Near>,
}

struct Link {
	seed: u32,
	mbid: Source,
	q: Q,
	strength: f32,
}

pub(super) fn of(db: &duckdb::Connection, pool: &[Scored]) -> hmerr::Result<HashMap<u32, Shape>> {
	db.execute_batch("create or replace temp table similar_pool (recording_id uinteger);")?;
	{
		let mut appender = db.appender("similar_pool")?;
		for scored in pool {
			appender.append_row(duckdb::params![scored.recording_id])?;
		}
		appender.flush()?;
	}

	let mut statement = db.prepare(&format!(
		r"
with pool_listen as (
	select ul.user_id, ul.recording_id, {PREFERENCE}(ul.plays, s.center) as preference
	from user_listen ul
	join user_stat s using (user_id)
	semi join similar_pool p using (recording_id)
)
select c.recording_id::bigint, d.recording_id::bigint, r.mbid::varchar, any_value(d.q)::utinyint,
	sum(c.preference * d.preference / p.spread)::float
from pool_listen c
join similar_declared_listen d using (user_id)
join similar_spread p on p.recording_id = d.recording_id
join recording r on r.recording_id = d.recording_id
where c.preference > 0 and d.preference > 0
group by c.recording_id, d.recording_id, r.mbid
"
	))?;

	let mut row = statement.query([])?;
	let mut link: HashMap<u32, Vec<Link>> = HashMap::new();

	while let Some(row) = row.next()? {
		let candidate: i64 = row.get(0)?;
		let seed: i64 = row.get(1)?;
		let mbid: String = row.get(2)?;
		let q: Q = row.get(3)?;
		let strength: f32 = row.get(4)?;

		let (Ok(candidate), Ok(seed), Ok(mbid)) =
			(u32::try_from(candidate), u32::try_from(seed), mbid.parse())
		else {
			continue;
		};

		link.entry(candidate).or_default().push(Link {
			seed,
			mbid,
			q,
			strength,
		});
	}

	Ok(link
		.into_iter()
		.map(|(candidate, link)| (candidate, shape(link)))
		.collect())
}

fn shape(mut link: Vec<Link>) -> Shape {
	let profile = Profile::of_strength(link.iter().map(|link| (link.seed, link.strength)));

	link.retain(|link| value::weight(link.q).abs() > f32::EPSILON);
	link.sort_by(|a, b| pull(b).total_cmp(&pull(a)));

	Shape {
		profile,
		near: link
			.into_iter()
			.take(NEAR)
			.map(|link| Near {
				mbid: link.mbid,
				q: link.q,
			})
			.collect(),
	}
}

fn pull(link: &Link) -> f32 {
	(link.strength * value::weight(link.q)).abs()
}

#[cfg(test)]
mod tests {
	use super::{
		super::{
			fixture::{self, LOVED, fans, mbid},
			predict::{self, Scored},
		},
		*,
	};

	const LIKED_SEED: u32 = 0;
	const AVOIDED_SEED: u32 = 1;
	const NEUTRAL_SEED: u32 = 2;
	const CANDIDATE: u32 = 3;

	#[test]
	fn the_same_profile_is_fully_redundant_and_a_disjoint_one_not_at_all() {
		let one = Profile::of_strength([(1, 2.0), (2, 1.0)]);
		let same = Profile::of_strength([(1, 4.0), (2, 2.0)]);
		let other = Profile::of_strength([(3, 1.0)]);

		assert!((one.cosine(&same) - 1.0).abs() < 1e-6);
		assert!(one.cosine(&other).abs() < 1e-6);
		assert!(Profile::default().cosine(&one).abs() < 1e-6);
	}

	#[test]
	fn a_candidate_is_near_what_its_listeners_love_strongest_pull_first_neutral_left_out() {
		let db = fixture::db(
			&[
				fans(0..5, LIKED_SEED, LOVED),
				fans(5..10, AVOIDED_SEED, LOVED),
				fans(0..2, NEUTRAL_SEED, LOVED),
				fans(0..10, CANDIDATE, LOVED),
			]
			.concat(),
			&[(LIKED_SEED, 3), (AVOIDED_SEED, 0), (NEUTRAL_SEED, 1)],
			&[],
		);
		let pool: Vec<Scored> = predict::candidates(&db).unwrap();

		let shape = of(&db, &pool).unwrap();
		let near: Vec<(Source, Q)> = shape
			.values()
			.next()
			.map(|shape| shape.near.iter().map(|near| (near.mbid, near.q)).collect())
			.unwrap_or_default();

		assert_eq!(near, vec![(mbid(AVOIDED_SEED), 0), (mbid(LIKED_SEED), 3)]);
	}

	#[test]
	fn an_empty_pool_has_no_profile() {
		let db = fixture::db(&fans(0..5, LIKED_SEED, LOVED), &[(LIKED_SEED, 3)], &[]);

		assert!(of(&db, &[]).unwrap().is_empty());
	}
}
