use ansi::abbrev::{B, D, F};

use super::{
	super::{
		dump::{self, Pending},
		progress,
	},
	work::Reach,
};

pub(super) fn left<'a>(pending: &'a [Pending], reach: &Reach) -> hmerr::Result<Vec<&'a Pending>> {
	let reached = dump::reach(&reach.covered)?;

	Ok(pending
		.iter()
		.filter(|pending| pending.past(reached, reach.through))
		.collect())
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
		super::fixture::{FOLDED, NEXT, WAITING, reaching, waiting},
		*,
	};

	#[test]
	fn what_a_previous_run_already_folded_is_never_offered_again() {
		let chain = vec![
			waiting(2594, 20_260_713_000_003, FOLDED),
			waiting(2595, 20_260_714_000_003, WAITING),
		];

		let left = left(&chain, &reaching(NEXT, None)).unwrap_or_default();

		assert_eq!(left.len(), 1);
		assert_eq!(
			dump::weight(&left),
			WAITING,
			"what is asked for is what is still to fetch"
		);
	}

	#[test]
	fn the_number_a_previous_run_read_through_decides_what_is_left_over_the_second_a_name_carries()
	{
		let chain = vec![
			waiting(2672, 20_260_922_000_003, FOLDED),
			waiting(2673, 20_260_922_000_002, WAITING),
		];

		let left = left(
			&chain,
			&reaching("2026-09-22 00:00:03.049829+00:00", Some(2672)),
		)
		.unwrap_or_default();

		assert_eq!(dump::weight(&left), WAITING);
	}
}
