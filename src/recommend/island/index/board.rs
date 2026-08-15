use std::ops::Deref;

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use indicatif::ProgressBar;

use super::progress::{self, Measure};

pub(super) struct Board {
	shown: Vec<Shown>,
}

pub(crate) struct Running {
	bar: ProgressBar,
}

struct Shown {
	title: &'static str,
	measure: Measure,
	bar: ProgressBar,
}

impl Board {
	pub(super) fn of(plan: &[(&'static str, Measure)]) -> hmerr::Result<Self> {
		let mut shown = Vec::with_capacity(plan.len());

		for (title, measure) in plan {
			shown.push(Shown {
				title,
				measure: *measure,
				bar: progress::waiting_bar(*measure, title)?,
			});
		}

		Ok(Self { shown })
	}

	pub(super) fn start(&self, title: &str) -> hmerr::Result<Running> {
		let shown = self
			.shown
			.iter()
			.find(|shown| shown.title == title)
			.ok_or_else(|| ge!(format!("{R}no bar was planned for {B}{title}{D}")))?;

		progress::started(&shown.bar, shown.title, shown.measure)?;

		Ok(Running {
			bar: shown.bar.clone(),
		})
	}

	pub(super) fn run<T>(
		&self,
		title: &str,
		work: impl FnOnce(&Running) -> hmerr::Result<T>,
	) -> hmerr::Result<T> {
		let bar = self.start(title)?;

		work(&bar)
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

	const PLAN: [(&str, Measure); 2] = [("first", Measure::Step(4)), ("then", Measure::Byte(1024))];

	fn board() -> Board {
		Board::of(&PLAN).unwrap_or_else(|_| unreachable!())
	}

	#[test]
	fn every_planned_stage_can_be_started() {
		let board = board();

		for (title, _) in PLAN {
			assert!(board.start(title).is_ok(), "{title}");
		}
	}

	#[test]
	fn a_stage_that_was_never_planned_has_no_bar_to_light_up() {
		assert!(board().start("nowhere").is_err());
	}

	#[test]
	fn a_stage_that_ran_is_finished_once_its_work_returns() {
		let board = board();

		let done = board.run("first", |bar| {
			assert!(!bar.is_finished());

			Ok(7)
		});

		assert_eq!(done.unwrap_or_default(), 7);
		assert!(board.start("first").is_ok());
	}
}
