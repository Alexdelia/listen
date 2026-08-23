pub(super) fn living(from: usize, count: usize, alive: impl Fn(usize) -> bool) -> Option<usize> {
	(0..count)
		.map(|step| (from + step) % count)
		.find(|turn| alive(*turn))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_turn_it_is_asked_from_comes_before_the_ones_after_it() {
		assert_eq!(living(2, 4, |_| true), Some(2));
	}

	#[test]
	fn a_turn_past_the_last_one_is_looked_for_from_the_first_again() {
		assert_eq!(living(3, 4, |turn| turn == 1), Some(1));
		assert_eq!(living(4, 4, |turn| turn == 0), Some(0));
	}

	#[test]
	fn nothing_alive_is_no_turn_at_all() {
		assert_eq!(living(0, 4, |_| false), None);
		assert_eq!(living(2, 0, |_| true), None);
	}
}
