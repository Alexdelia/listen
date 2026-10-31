use chrono::DateTime;
use console::style;

use crate::format::{DATE_FORMAT, TIME_FORMAT};

use super::Held;

pub(super) fn another_dump(unpacked: &str) {
	println!(
		"{}{}{}",
		style(
			"the dump unpacked is not the one the counts were read off, \
		reading "
		)
		.yellow(),
		style(&unpacked).bold().yellow(),
		style(" up and asking for every incremental since").yellow()
	);
}

pub(super) fn merged_in() {
	println!(
		"{}",
		style(
			"the cached count cannot tell the dump from what was folded onto it, \
		reading the dump up again and asking for every incremental since"
		)
		.yellow()
	);
}

pub(super) fn stuck_at(reached: &str) {
	println!(
		"{}{}{}",
		style("the counts stopped at ").yellow(),
		style(&reached).bold().yellow(),
		style(
			", which no dump can be held against, \
		reading the dump up again and asking for every incremental since"
		)
		.yellow()
	);
}

pub(super) fn reading() {
	println!(
		"{}",
		style("reading own listen off the unpacked dump, once per dump").dim()
	);
}

pub(super) fn announce(username: &str, held: &Held) -> miette::Result<()> {
	println!(
		"{} recording off the dump for {}, covering up to \
		{} {}{}{}\n",
		style(&held.recording()).bold().green(),
		style(&username).bold(),
		style(&at(held.reached_at())).bold(),
		style(format!(
			"({day} day ago, last listen {last}, \
		",
			day = held.ago()?,
			last = at(held.covered)
		))
		.dim(),
		style("--refresh").dim().bold(),
		style(" to read the dump again)").dim()
	);

	Ok(())
}

fn at(second: i64) -> String {
	DateTime::from_timestamp(second, 0)
		.map(|at| {
			at.format(&format!("{DATE_FORMAT} {TIME_FORMAT}"))
				.to_string()
		})
		.unwrap_or_default()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_timestamp_is_told_as_a_date_and_a_time() {
		assert_eq!(at(1_783_802_344), "2026-07-11 20:39".to_string());
	}
}
