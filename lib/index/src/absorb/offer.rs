use ansi::abbrev::{B, D, F, Y};

use super::super::{
	decide::Decide,
	dump::{self, Pending},
	progress,
};

pub(super) fn left<'a>(pending: &'a [Pending], covered: &str) -> hmerr::Result<Vec<&'a Pending>> {
	let reached = dump::reach(covered)?;

	Ok(pending
		.iter()
		.filter(|pending| pending.reach > reached)
		.collect())
}

pub(super) fn offered(left: &[&Pending], decide: &dyn Decide) -> hmerr::Result<bool> {
	if left.is_empty() {
		return Ok(true);
	}

	progress::say(format!(
		"\n{F}absorbing {B}{count}{D}{F} incremental dump, {B}{Y}{size}{D}{F}, \
		each read once then deleted{D}",
		count = left.len(),
		size = progress::bytes(dump::weight(left))
	));

	progress::confirm(decide, "download", true)
}

pub(super) fn resuming(pending: &[Pending], left: &[&Pending]) {
	if left.len() == pending.len() {
		return;
	}

	progress::say(format!(
		"{F}{B}{done}{D}{F} of them already absorbed by a previous run, {B}{left}{D}{F} left{D}",
		done = pending.len() - left.len(),
		left = left.len()
	));
}

#[cfg(test)]
mod tests {
	use super::{
		super::{
			super::decide::Refuse,
			fixture::{FOLDED, NEXT, WAITING, waiting},
		},
		*,
	};

	#[test]
	fn what_a_previous_run_already_folded_is_never_offered_again() {
		let chain = vec![
			waiting(20_260_713_000_003, FOLDED),
			waiting(20_260_714_000_003, WAITING),
		];

		let left = left(&chain, NEXT).unwrap_or_default();

		assert_eq!(left.len(), 1);
		assert_eq!(
			dump::weight(&left),
			WAITING,
			"what is asked for is what is still to fetch"
		);
	}

	#[test]
	fn a_chain_a_previous_run_folded_whole_asks_for_nothing() {
		assert!(offered(&[], &Refuse).unwrap_or_default());
	}

	#[test]
	fn a_chain_left_to_fetch_absorbs_nothing_when_the_answer_is_no() {
		let waiting = waiting(20_260_714_000_003, WAITING);
		let left = vec![&waiting];

		assert!(!offered(&left, &Refuse).unwrap_or(true));
	}
}
