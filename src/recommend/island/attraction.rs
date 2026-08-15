use crate::declaration::value;

pub(super) const WEIGHT: &str = "attraction_weight";

const VALUE: &str = "attraction_value";

const LOW_QUANTILE: f32 = 0.05;
const CENTER_QUANTILE: f32 = 0.5;
const HIGH_QUANTILE: f32 = 0.99;

const CENTER_ROOM_RATIO: f32 = 2.0;
const SPAN_PER_ROOM: f32 = 3.0;
const SPAN_HALF_RATIO: f32 = 3.0;
const REACH: f32 = 50.0;
const REPEL: f32 = 15.0;
const EPSILON: f32 = 1e-6;

const ROOM: &str = "attraction_room";
const CENTER: &str = "attraction_center";

pub(super) fn declare(db: &duckdb::Connection) -> hmerr::Result<()> {
	let neutral = f32::from(value::NEUTRAL);
	let room = CENTER_ROOM_RATIO.ln();
	let span_half = SPAN_HALF_RATIO.ln();

	db.execute_batch(&format!(
		r"
create or replace macro {ROOM}(low, high) as
	least({room}, (high - low) / {SPAN_PER_ROOM});
create or replace macro {CENTER}(center, low, high) as
	least(
		greatest(center, low + {ROOM}(low, high)),
		high - {ROOM}(low, high)
	);
create or replace macro {VALUE}(plays, center, low, high) as
	{neutral} + (
		case when ln(greatest(plays, 1)) >= {CENTER}(center, low, high)
			then {REACH} * least(
				(ln(greatest(plays, 1)) - {CENTER}(center, low, high))
					/ greatest(high - {CENTER}(center, low, high), {EPSILON}),
				1
			)
			else {REPEL} * greatest(
				(ln(greatest(plays, 1)) - {CENTER}(center, low, high))
					/ greatest({CENTER}(center, low, high) - low, {EPSILON}),
				-1
			)
		end
	) * ((high - low) / ((high - low) + {span_half}));
create or replace macro {WEIGHT}(plays, center, low, high) as
	({VALUE}(plays, center, low, high) - {neutral}) / {neutral};
"
	))?;

	Ok(())
}

pub(super) fn stat(source: &str) -> String {
	format!(
		r"
with occurrence as (
	select user_id, plays, count(*)::bigint as recording
	from {source}
	group by 1, 2
),
whole as (
	select user_id, sum(recording) as recording, sum(plays::bigint * recording) as listen
	from occurrence
	group by 1
),
running as (
	select user_id,
		ln(greatest(plays, 1)) as x,
		sum(plays::bigint * recording) over w as reached
	from occurrence
	window w as (partition by user_id order by plays rows between unbounded preceding and current row)
)
select
	w.user_id::uinteger as user_id,
	min(case when r.reached >= {CENTER_QUANTILE} * w.listen then r.x end)::float as center,
	min(case when r.reached >= {LOW_QUANTILE} * w.listen then r.x end)::float as low,
	min(case when r.reached >= {HIGH_QUANTILE} * w.listen then r.x end)::float as high,
	w.recording::uinteger as recording
from running r
join whole w using (user_id)
group by 1, 5
"
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	const ROW_LIMIT: usize = 10000;

	struct Stat {
		center: f32,
		low: f32,
		high: f32,
	}

	fn db() -> duckdb::Connection {
		let db = duckdb::Connection::open_in_memory().unwrap();
		declare(&db).unwrap();

		db
	}

	fn stat_of(db: &duckdb::Connection, library: &[(u32, u32)]) -> Stat {
		let row = library
			.iter()
			.map(|(plays, recording)| format!("(0, {plays}, {recording})"))
			.collect::<Vec<_>>()
			.join(",");

		db.query_row(
			&format!(
				"select center, low, high from ({stat})",
				stat = stat(&format!(
					"(select t.user_id, t.plays \
					from (values {row}) as t(user_id, plays, recording) \
					join range({ROW_LIMIT}) as r on r.range < t.recording)"
				))
			),
			[],
			|row| {
				Ok(Stat {
					center: row.get(0)?,
					low: row.get(1)?,
					high: row.get(2)?,
				})
			},
		)
		.unwrap()
	}

	fn value(db: &duckdb::Connection, plays: u32, stat: &Stat) -> f32 {
		db.query_row(
			&format!(
				"select {VALUE}({plays}, {center}, {low}, {high})::float",
				center = stat.center,
				low = stat.low,
				high = stat.high,
			),
			[],
			|row| row.get(0),
		)
		.unwrap()
	}

	fn neutral() -> f32 {
		f32::from(value::NEUTRAL)
	}

	fn whale(db: &duckdb::Connection) -> Stat {
		stat_of(db, &[(1, 900), (5, 60), (30, 30), (200, 9), (2000, 1)])
	}

	fn focused(db: &duckdb::Connection) -> Stat {
		stat_of(db, &[(11, 9), (100, 1)])
	}

	#[test]
	fn a_user_who_repeats_everything_the_same_never_leaves_neutral() {
		let db = db();
		let flat = stat_of(&db, &[(7, 40)]);

		for plays in [1, 7, 500] {
			let value = value(&db, plays, &flat);

			assert!(
				(value - neutral()).abs() < 1e-3,
				"{plays} play reads {value}"
			);
		}
	}

	#[test]
	fn the_single_play_tail_of_a_wide_library_sits_below_neutral() {
		let db = db();

		assert!(value(&db, 1, &whale(&db)) < neutral());
	}

	#[test]
	fn an_inferred_dislike_never_sinks_below_a_declared_one() {
		let db = db();

		assert!(value(&db, 1, &whale(&db)) >= f32::from(value::from_q(0)));
	}

	#[test]
	fn a_much_repeated_recording_of_a_wide_library_still_falls_short_of_its_top() {
		let db = db();
		let whale = whale(&db);
		let much = value(&db, 1000, &whale);

		assert!(much > 70.0, "{much}");
		assert!(much < value(&db, 2000, &whale), "{much}");
	}

	#[test]
	fn the_most_repeated_recording_of_a_focused_user_reaches_for_the_top() {
		let db = db();

		assert!(value(&db, 100, &focused(&db)) > 75.0);
	}

	#[test]
	fn the_top_of_a_thin_library_beats_a_middling_entry_of_a_huge_one() {
		let db = db();

		assert!(value(&db, 100, &focused(&db)) > value(&db, 30, &whale(&db)));
	}

	#[test]
	fn two_plays_carry_a_verdict_instead_of_being_dropped() {
		let db = db();
		let whale = whale(&db);

		assert!(value(&db, 2, &whale) > value(&db, 1, &whale));
		assert!(value(&db, 2, &whale) < neutral());
	}

	#[test]
	fn a_neutral_recording_neither_attracts_nor_repels() {
		let db = db();
		let flat = stat_of(&db, &[(4, 40)]);
		let weight: f32 = db
			.query_row(
				&format!(
					"select {WEIGHT}(4, {center}, {low}, {high})::float",
					center = flat.center,
					low = flat.low,
					high = flat.high
				),
				[],
				|row| row.get(0),
			)
			.unwrap();

		assert!(weight.abs() < 1e-3, "{weight}");
	}

	#[test]
	fn a_library_whose_listening_hides_in_its_tail_keeps_room_above_its_center() {
		let db = db();
		let concentrated = stat_of(&db, &[(1, 900), (50, 10)]);

		assert!(value(&db, 1, &concentrated) < neutral());
		assert!(value(&db, 50, &concentrated) > 80.0);
	}
}
