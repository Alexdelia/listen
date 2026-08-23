use std::ops::Deref;

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use indicatif::ProgressBar;

use super::progress::{self, Measure};

pub(super) trait Planned: Copy + Eq {
	type Cost;

	fn title(self) -> &'static str;

	fn measure(self, cost: &Self::Cost) -> Measure;
}

pub(super) struct Board<S> {
	shown: Vec<Shown<S>>,
}

pub(crate) struct Running {
	bar: ProgressBar,
}

struct Shown<S> {
	stage: S,
	measure: Measure,
	bar: ProgressBar,
}

impl<S: Planned> Board<S> {
	pub(super) fn of(plan: &[S], cost: &S::Cost) -> hmerr::Result<Self> {
		let mut shown = Vec::with_capacity(plan.len());

		for stage in plan {
			let measure = stage.measure(cost);

			shown.push(Shown {
				stage: *stage,
				measure,
				bar: progress::waiting_bar(measure, stage.title())?,
			});
		}

		Ok(Self { shown })
	}

	pub(super) fn start(&self, stage: S) -> hmerr::Result<Running> {
		let shown = self
			.shown
			.iter()
			.find(|shown| shown.stage == stage)
			.ok_or_else(|| {
				ge!(format!(
					"{R}no bar was planned for {B}{title}{D}",
					title = stage.title()
				))
			})?;

		progress::started(&shown.bar, stage.title(), shown.measure)?;

		Ok(Running {
			bar: shown.bar.clone(),
		})
	}

	pub(super) fn run<T>(
		&self,
		stage: S,
		work: impl FnOnce(&Running) -> hmerr::Result<T>,
	) -> hmerr::Result<T> {
		let bar = self.start(stage)?;

		work(&bar)
	}
}

impl<S> Drop for Board<S> {
	fn drop(&mut self) {
		for shown in &self.shown {
			progress::ended(&shown.bar);
		}
	}
}

impl Deref for Running {
	type Target = ProgressBar;

	fn deref(&self) -> &Self::Target {
		&self.bar
	}
}

impl Drop for Running {
	fn drop(&mut self) {
		progress::ended(&self.bar);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone, Copy, PartialEq, Eq)]
	enum Stage {
		First,
		Then,
		Nowhere,
	}

	const PLAN: [Stage; 2] = [Stage::First, Stage::Then];

	impl Planned for Stage {
		type Cost = ();

		fn title(self) -> &'static str {
			match self {
				Self::First => "first",
				Self::Then => "then",
				Self::Nowhere => "nowhere",
			}
		}

		fn measure(self, (): &()) -> Measure {
			match self {
				Self::First | Self::Nowhere => Measure::Step(4),
				Self::Then => Measure::Byte(1024),
			}
		}
	}

	fn board() -> Board<Stage> {
		Board::of(&PLAN, &()).unwrap_or_else(|_| unreachable!())
	}

	#[test]
	fn every_planned_stage_can_be_started() {
		let board = board();

		for stage in PLAN {
			assert!(board.start(stage).is_ok(), "{}", stage.title());
		}
	}

	#[test]
	fn a_stage_that_was_never_planned_has_no_bar_to_light_up() {
		assert!(board().start(Stage::Nowhere).is_err());
	}

	#[test]
	fn a_stage_that_ran_is_finished_once_its_work_returns() {
		let board = board();

		let done = board.run(Stage::First, |bar| {
			assert!(!bar.is_finished());

			Ok(7)
		});

		assert_eq!(done.unwrap_or_default(), 7);
		assert!(board.start(Stage::First).is_ok());
	}
}
