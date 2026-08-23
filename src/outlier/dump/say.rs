use ansi::{
	DIM,
	abbrev::{B, D, F, G, Y},
};
use chrono::DateTime;

use crate::format::{DATE_FORMAT, TIME_FORMAT};

use super::Held;

pub(super) fn another_dump(unpacked: &str) {
	println!(
		"{Y}the dump unpacked is not the one the counts were read off, \
		reading {B}{unpacked}{D}{Y} up and asking for every incremental since{D}"
	);
}

pub(super) fn merged_in() {
	println!(
		"{Y}the cached count cannot tell the dump from what was folded onto it, \
		reading the dump up again and asking for every incremental since{D}"
	);
}

pub(super) fn stuck_at(reached: &str) {
	println!(
		"{Y}the counts stopped at {B}{reached}{D}{Y}, which no dump can be held against, \
		reading the dump up again and asking for every incremental since{D}"
	);
}

pub(super) fn reading() {
	println!("{F}reading own listen off the unpacked dump, once per dump{D}");
}

pub(super) fn announce(username: &str, held: &Held) -> hmerr::Result<()> {
	println!(
		"{B}{G}{count}{D} recording off the dump for {B}{username}{D}, covering up to \
		{B}{reached}{D} {DIM}({day} day ago, last listen {last}, \
		{B}--refresh{D}{DIM} to read the dump again){D}\n",
		count = held.counted().len(),
		reached = at(held.reached_at()),
		day = held.ago()?,
		last = at(held.covered)
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
