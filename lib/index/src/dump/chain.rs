use std::{
	path::Path,
	sync::mpsc::{self, Receiver},
	thread,
};

use super::{
	super::{board::Running, parallel},
	incremental::{self, Incremental, Pending},
};

const AHEAD: usize = 1;
const IN_HAND: usize = AHEAD + 1;
const UNPACKING: usize = 2;

pub(crate) const AT_ONCE: u64 = (IN_HAND + UNPACKING) as u64;

pub(crate) struct Bar<'b> {
	pub downloading: &'b Running,
	pub verifying: &'b Running,
	pub unpacking: &'b Running,
	pub folding: &'b Running,
}

pub(crate) fn each(
	root: &Path,
	pending: &[&Pending],
	bar: &Bar<'_>,
	mut fold: impl FnMut(&Incremental) -> hmerr::Result<()>,
) -> hmerr::Result<()> {
	piped(
		pending.len(),
		|step| incremental::pull(root, pending[step], bar.downloading, bar.verifying),
		|step| one(root, pending[step], bar, &mut fold),
	)
}

fn one(
	root: &Path,
	pending: &Pending,
	bar: &Bar<'_>,
	fold: &mut impl FnMut(&Incremental) -> hmerr::Result<()>,
) -> hmerr::Result<()> {
	let incremental = incremental::opened(root, pending, bar.unpacking)?;

	let folded = fold(&incremental);
	bar.folding.inc(1);

	incremental::release(&incremental)?;

	folded
}

fn piped(
	count: usize,
	produce: impl Fn(usize) -> hmerr::Result<()> + Send,
	mut consume: impl FnMut(usize) -> hmerr::Result<()>,
) -> hmerr::Result<()> {
	let (send, receive) = mpsc::sync_channel(AHEAD);

	thread::scope(|scope| {
		let producing = scope.spawn(move || {
			for step in 0..count {
				produce(step).map_err(|e| e.to_string())?;

				if send.send(step).is_err() {
					break;
				}
			}

			Ok(())
		});

		let consumed = consume_each(&receive, &mut consume);
		drop(receive);

		consumed.and_then(|()| parallel::joined(producing.join()))
	})
}

fn consume_each(
	receive: &Receiver<usize>,
	consume: &mut impl FnMut(usize) -> hmerr::Result<()>,
) -> hmerr::Result<()> {
	while let Ok(step) = receive.recv() {
		consume(step)?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::sync::{
		Mutex, PoisonError,
		atomic::{AtomicUsize, Ordering},
	};

	use hmerr::ge;

	use super::*;

	const CHAIN: usize = 20;

	fn stopped() -> hmerr::Result<()> {
		Err(ge!("stopped".to_string()).into())
	}

	fn keep(held: &Mutex<Vec<usize>>, step: usize) {
		held.lock()
			.unwrap_or_else(PoisonError::into_inner)
			.push(step);
	}

	fn taken(held: &Mutex<Vec<usize>>) -> Vec<usize> {
		held.lock().unwrap_or_else(PoisonError::into_inner).clone()
	}

	#[test]
	fn every_step_is_produced_then_consumed_in_the_order_it_was_given() {
		let produced = Mutex::new(Vec::new());
		let consumed = Mutex::new(Vec::new());

		let done = piped(
			5,
			|step| {
				keep(&produced, step);

				Ok(())
			},
			|step| {
				keep(&consumed, step);

				Ok(())
			},
		);

		assert!(done.is_ok());
		assert_eq!(taken(&produced), vec![0, 1, 2, 3, 4]);
		assert_eq!(taken(&consumed), vec![0, 1, 2, 3, 4]);
	}

	#[test]
	fn producing_runs_ahead_of_consuming_but_never_far_ahead() {
		let ahead = AtomicUsize::new(0);
		let peak = AtomicUsize::new(0);

		let done = piped(
			CHAIN,
			|_| {
				let now = ahead.fetch_add(1, Ordering::SeqCst) + 1;
				peak.fetch_max(now, Ordering::SeqCst);

				Ok(())
			},
			|_| {
				ahead.fetch_sub(1, Ordering::SeqCst);

				Ok(())
			},
		);

		assert!(done.is_ok());
		let peak = peak.load(Ordering::SeqCst);
		assert!(peak <= AHEAD + 2, "{peak}");
		assert!(
			u64::try_from(peak + 1).unwrap_or(u64::MAX) <= AT_ONCE,
			"{peak} archive at once, and the one unpacking beside them, outgrows what is reserved"
		);
	}

	#[test]
	fn a_step_that_cannot_be_produced_stops_the_chain() {
		let consumed = Mutex::new(Vec::new());

		let done = piped(
			CHAIN,
			|step| {
				if step == 2 {
					return stopped();
				}

				Ok(())
			},
			|step| {
				keep(&consumed, step);

				Ok(())
			},
		);

		assert!(done.is_err());
		assert!(taken(&consumed).iter().all(|step| *step < 2));
	}

	#[test]
	fn a_step_that_cannot_be_consumed_stops_the_producer_too() {
		let produced = AtomicUsize::new(0);

		let done = piped(
			CHAIN,
			|_| {
				produced.fetch_add(1, Ordering::SeqCst);

				Ok(())
			},
			|_| stopped(),
		);

		assert!(done.is_err());
		assert!(
			produced.load(Ordering::SeqCst) < CHAIN,
			"the producer kept going with nowhere to send"
		);
	}

	#[test]
	fn what_a_failed_step_said_is_what_comes_back() {
		let said = piped(1, |_| stopped(), |_| Ok(()))
			.err()
			.map(|e| format!("{e}"))
			.unwrap_or_default();

		assert!(said.contains("stopped"), "{said}");
	}

	#[test]
	fn an_empty_chain_is_not_an_error() {
		assert!(piped(0, |_| Ok(()), |_| Ok(())).is_ok());
	}
}
