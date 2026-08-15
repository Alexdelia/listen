use std::{
	ops::Deref,
	sync::{Condvar, Mutex, MutexGuard, PoisonError},
	thread,
};

use indicatif::ProgressBar;

use super::super::parallel;

pub(super) struct Lane {
	free: Mutex<Vec<duckdb::Connection>>,
	ready: Condvar,
}

pub(crate) struct Held<'a> {
	lane: &'a Lane,
	db: Option<duckdb::Connection>,
}

impl Lane {
	pub(super) fn of(db: duckdb::Connection, width: usize) -> hmerr::Result<Self> {
		let mut free = Vec::with_capacity(width.max(1));
		for _ in 1..width {
			free.push(db.try_clone()?);
		}
		free.push(db);

		Ok(Self {
			free: Mutex::new(free),
			ready: Condvar::new(),
		})
	}

	pub(super) fn take(&self) -> Held<'_> {
		let mut free = self.pool();

		loop {
			if let Some(db) = free.pop() {
				return Held {
					lane: self,
					db: Some(db),
				};
			}

			free = self
				.ready
				.wait(free)
				.unwrap_or_else(PoisonError::into_inner);
		}
	}

	pub(super) fn spread<T: Sync>(
		&self,
		unit: &[T],
		bar: &ProgressBar,
		run: impl Fn(&duckdb::Connection, &T) -> hmerr::Result<()> + Sync,
	) -> hmerr::Result<()> {
		thread::scope(|scope| {
			let running: Vec<_> = unit
				.iter()
				.map(|unit| {
					scope.spawn(|| {
						let done = run(&self.take(), unit).map_err(|e| e.to_string());
						bar.inc(1);

						done
					})
				})
				.collect();

			running
				.into_iter()
				.map(|running| parallel::joined(running.join()))
				.collect::<hmerr::Result<Vec<()>>>()
		})?;

		Ok(())
	}

	fn pool(&self) -> MutexGuard<'_, Vec<duckdb::Connection>> {
		self.free.lock().unwrap_or_else(PoisonError::into_inner)
	}
}

impl Deref for Held<'_> {
	type Target = duckdb::Connection;

	fn deref(&self) -> &Self::Target {
		self.db.as_ref().unwrap_or_else(|| unreachable!())
	}
}

impl Drop for Held<'_> {
	fn drop(&mut self) {
		if let Some(db) = self.db.take() {
			self.lane.pool().push(db);
			self.lane.ready.notify_one();
		}
	}
}

#[cfg(test)]
mod tests {
	use std::sync::atomic::{AtomicUsize, Ordering};

	use hmerr::ge;

	use super::*;

	fn lane(width: usize) -> Lane {
		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		Lane::of(db, width).unwrap_or_else(|_| unreachable!())
	}

	fn bar() -> ProgressBar {
		ProgressBar::hidden()
	}

	#[test]
	fn every_unit_is_run() {
		let lane = lane(4);
		let unit: Vec<usize> = (0..32).collect();
		let run = AtomicUsize::new(0);

		let done = lane.spread(&unit, &bar(), |_, _| {
			run.fetch_add(1, Ordering::Relaxed);

			Ok(())
		});

		assert!(done.is_ok());
		assert_eq!(run.load(Ordering::Relaxed), 32);
	}

	#[test]
	fn no_more_units_run_at_once_than_there_are_lanes() {
		let lane = lane(3);
		let unit: Vec<usize> = (0..24).collect();
		let running = AtomicUsize::new(0);
		let peak = AtomicUsize::new(0);

		let _ = lane.spread(&unit, &bar(), |_, _| {
			let at_once = running.fetch_add(1, Ordering::SeqCst) + 1;
			peak.fetch_max(at_once, Ordering::SeqCst);
			std::thread::sleep(std::time::Duration::from_millis(1));
			running.fetch_sub(1, Ordering::SeqCst);

			Ok(())
		});

		assert!(peak.load(Ordering::SeqCst) <= 3, "{peak:?}");
	}

	#[test]
	fn what_one_unit_said_when_it_failed_fails_the_spread() {
		let lane = lane(2);
		let unit: Vec<usize> = (0..8).collect();

		let said = lane
			.spread(&unit, &bar(), |_, unit| {
				if *unit == 5 {
					return Err(ge!("unit 5 failed".to_string()).into());
				}

				Ok(())
			})
			.err()
			.map(|e| format!("{e}"))
			.unwrap_or_default();

		assert!(said.contains("unit 5 failed"), "{said}");
	}

	#[test]
	fn a_lane_is_handed_back_once_its_unit_is_done() {
		let lane = lane(1);
		let unit: Vec<usize> = (0..4).collect();

		assert!(lane.spread(&unit, &bar(), |_, _| Ok(())).is_ok());
		assert_eq!(lane.pool().len(), 1);
	}
}
