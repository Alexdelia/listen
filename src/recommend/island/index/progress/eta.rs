use std::{
	fmt,
	time::{Duration, Instant},
};

use indicatif::{HumanDuration, ProgressState, style::ProgressTracker};

pub(super) const KEY: &str = "eta";

const NOTHING: &str = "-";

#[derive(Clone)]
pub(super) struct Eta {
	start: Instant,
	last: Instant,
	done: u64,
}

impl Eta {
	pub(super) fn new() -> Self {
		let now = Instant::now();

		Self {
			start: now,
			last: now,
			done: 0,
		}
	}

	fn left(&self, len: Option<u64>, pos: u64) -> Option<Duration> {
		let left = u128::from(len?.saturating_sub(pos));

		if left == 0 {
			return Some(Duration::ZERO);
		}

		if self.done == 0 {
			return None;
		}

		let took = self.last.saturating_duration_since(self.start).as_nanos();

		u64::try_from(took * left / u128::from(self.done))
			.ok()
			.map(Duration::from_nanos)
	}
}

impl ProgressTracker for Eta {
	fn clone_box(&self) -> Box<dyn ProgressTracker> {
		Box::new(self.clone())
	}

	fn tick(&mut self, state: &ProgressState, now: Instant) {
		if state.pos() == self.done {
			return;
		}

		self.done = state.pos();
		self.last = now;
	}

	fn reset(&mut self, _: &ProgressState, now: Instant) {
		self.start = now;
		self.last = now;
		self.done = 0;
	}

	fn write(&self, state: &ProgressState, w: &mut dyn fmt::Write) {
		let _ = match self.left(state.len(), state.pos()) {
			Some(left) => write!(w, "{:#}", HumanDuration(left)),
			None => write!(w, "{NOTHING}"),
		};
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn after(second: u64, done: u64) -> Eta {
		let start = Instant::now();

		Eta {
			start,
			last: start + Duration::from_secs(second),
			done,
		}
	}

	#[test]
	fn what_is_left_is_what_the_finished_steps_took() {
		assert_eq!(
			after(60, 4).left(Some(48), 4),
			Some(Duration::from_mins(11))
		);
	}

	#[test]
	fn a_stage_that_finished_nothing_yet_cannot_say_when_it_ends() {
		assert_eq!(after(600, 0).left(Some(48), 0), None);
		assert_eq!(Eta::new().left(Some(48), 0), None);
	}

	#[test]
	fn a_stage_with_nothing_left_ends_now() {
		assert_eq!(after(60, 48).left(Some(48), 48), Some(Duration::ZERO));
	}

	#[test]
	fn a_finished_stage_ends_now_even_though_its_last_step_went_unrecorded() {
		assert_eq!(after(60, 2).left(Some(4), 4), Some(Duration::ZERO));
	}

	#[test]
	fn how_fast_it_goes_is_read_off_the_recorded_steps_and_how_far_off_the_live_one() {
		assert_eq!(
			after(60, 2).left(Some(48), 3),
			Some(Duration::from_secs(45 * 30))
		);
	}

	#[test]
	fn a_stage_of_unknown_length_cannot_say_when_it_ends() {
		assert_eq!(after(60, 4).left(None, 4), None);
	}

	#[test]
	fn the_time_a_stage_spent_waiting_on_its_first_step_is_not_counted_against_it() {
		let waited = after(60, 4);
		let hurried = Eta {
			start: waited.start,
			last: waited.last,
			done: 8,
		};

		assert!(hurried.left(Some(48), 8) < waited.left(Some(48), 4));
	}
}
