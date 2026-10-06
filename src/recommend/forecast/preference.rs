use crate::declaration::{Q_MAX, value};

pub(super) const ONE_PLAY: f32 = 45.0;

pub(super) const PREFERENCE: &str = "forecast_preference";

const VALUE: &str = "forecast_value";

pub(super) fn declare(db: &duckdb::Connection) -> hmerr::Result<()> {
	let neutral = f32::from(value::NEUTRAL);
	let top = f32::from(value::from_q(Q_MAX));

	db.execute_batch(&format!(
		r"
create or replace macro {VALUE}(plays, center) as
	case when plays <= 1 then {ONE_PLAY}
	else least({top}, {ONE_PLAY} + ({top} - {ONE_PLAY}) * ln(plays) / greatest(center, ln(2)))
	end;
create or replace macro {PREFERENCE}(plays, center) as
	({VALUE}(plays, center) - {neutral}) / {neutral};
"
	))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn preference(plays: u16, center_play: f32) -> f32 {
		let db = duckdb::Connection::open_in_memory().unwrap();
		declare(&db).unwrap();

		db.query_row(
			&format!(
				"select {PREFERENCE}({plays}, {center})::float",
				center = center_play.ln()
			),
			[],
			|row| row.get(0),
		)
		.unwrap()
	}

	#[test]
	fn a_single_play_sits_at_the_one_play_value() {
		let expected = (ONE_PLAY - 50.0) / 50.0;

		assert!((preference(1, 10.0) - expected).abs() < 1e-6);
	}

	#[test]
	fn a_listeners_usual_play_count_and_above_is_full_preference() {
		assert!((preference(10, 10.0) - 1.0).abs() < 1e-6);
		assert!((preference(500, 10.0) - 1.0).abs() < 1e-6);
	}

	#[test]
	fn more_plays_never_mean_less_preference() {
		let mut last = f32::MIN;

		for plays in 1..=20 {
			let now = preference(plays, 10.0);
			assert!(now >= last, "{plays} plays: {now} under {last}");
			last = now;
		}
	}

	#[test]
	fn a_listener_who_plays_everything_once_is_full_at_two_plays() {
		assert!((preference(2, 1.0) - 1.0).abs() < 1e-6);
	}
}
