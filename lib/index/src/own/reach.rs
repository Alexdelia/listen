use super::super::dump;

pub(super) fn lost(reached: &str, start: &str) -> bool {
	dump::reach(start)
		.ok()
		.zip(dump::reach(reached).ok())
		.is_some_and(|(start, reached)| start > reached)
}

pub(super) fn behind(reached: &str, start: &str) -> bool {
	dump::reach(start)
		.ok()
		.zip(dump::reach(reached).ok())
		.is_some_and(|(start, reached)| start < reached)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_dump_starting_where_the_count_stopped_leaves_nothing_out_of_reach() {
		assert!(!lost(
			"2026-08-21 00:00:03.155180+00:00",
			"2026-08-21 00:00:03.155180+00:00"
		));
		assert!(!lost(
			"2026-08-22 00:00:02.641933+00:00",
			"2026-08-21 00:00:03.155180+00:00"
		));
	}

	#[test]
	fn a_dump_starting_past_where_the_count_stopped_leaves_a_window_out_of_reach() {
		assert!(lost(
			"2026-07-12 00:00:04.001868+00:00",
			"2026-07-23 00:00:03.690928+00:00"
		));
	}

	#[test]
	fn a_dump_starting_before_the_count_stopped_is_behind_it() {
		assert!(behind(
			"2026-07-12 00:00:04.001868+00:00",
			"2026-07-12 00:00:02.000000+00:00"
		));
		assert!(!behind(
			"2026-07-12 00:00:04.001868+00:00",
			"2026-07-12 00:00:04.001868+00:00"
		));
		assert!(!behind(
			"2026-07-12 00:00:04.001868+00:00",
			"2026-07-23 00:00:03.690928+00:00"
		));
	}
}
