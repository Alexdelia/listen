use std::num::NonZeroUsize;

use ansi::abbrev::{B, D, F};

use super::super::super::progress;

const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;

const MEMORY_PERCENT: u64 = 75;
const LANE_MEMORY: u64 = 2 * GIB;
const CHUNK: u64 = 4 * GIB;

pub(super) struct Size {
	pub thread: usize,
	pub lane: usize,
	pub batch: usize,
	memory: u64,
}

impl Size {
	pub(super) fn of(offered: u64, dump: u64, shard: usize) -> Self {
		let thread = thread();
		let memory = offered * MEMORY_PERCENT / 100;

		Self {
			thread,
			lane: lane(memory, thread),
			batch: batch(dump, shard),
			memory,
		}
	}

	pub(super) fn limit(&self) -> String {
		format!("{mib}MiB", mib = (self.memory / MIB).max(1))
	}

	pub(super) fn tell(&self) {
		progress::say(format!(
			"{F}{B}{thread}{D}{F} thread, {B}{memory}{D}{F} memory, \
			{B}{lane}{D}{F} query at once, dump read in {B}{batch}{D}{F} slice{D}",
			thread = self.thread,
			memory = progress::bytes(self.memory),
			lane = self.lane,
			batch = self.batch
		));
	}
}

pub(super) fn offered(db: &duckdb::Connection) -> u64 {
	let said: String = db
		.query_row("select current_setting('memory_limit')", [], |row| {
			row.get(0)
		})
		.unwrap_or_default();

	parse(&said).unwrap_or(LANE_MEMORY)
}

fn thread() -> usize {
	std::thread::available_parallelism().map_or(1, NonZeroUsize::get)
}

fn lane(memory: u64, thread: usize) -> usize {
	usize::try_from(memory / LANE_MEMORY)
		.unwrap_or(thread)
		.clamp(1, thread)
}

fn batch(dump: u64, shard: usize) -> usize {
	usize::try_from(dump.div_ceil(CHUNK))
		.unwrap_or(shard)
		.clamp(1, shard.max(1))
}

fn parse(said: &str) -> Option<u64> {
	let cut = said.find(|c: char| !c.is_ascii_digit() && c != '.')?;
	let (amount, unit) = said.split_at(cut);
	let scale = scale(unit.trim())?;

	let (whole, fraction) = amount.split_once('.').unwrap_or((amount, ""));
	let whole: u64 = whole.parse().ok()?;
	let tenth: u64 = match fraction.get(..1) {
		Some(tenth) => tenth.parse().ok()?,
		None => 0,
	};

	Some(
		whole
			.saturating_mul(scale)
			.saturating_add(tenth * scale / 10),
	)
}

fn scale(unit: &str) -> Option<u64> {
	match unit {
		"B" | "bytes" => Some(1),
		"KiB" => Some(1 << 10),
		"MiB" => Some(MIB),
		"GiB" => Some(GIB),
		"TiB" => Some(1 << 40),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn duckdb_says_what_it_would_take_and_it_reads_back_in_bytes() {
		assert_eq!(parse("12.4 GiB"), Some(13_314_398_617));
		assert_eq!(parse("9.3 GiB"), Some(9_985_798_963));
		assert_eq!(parse("512.0 MiB"), Some(512 * MIB));
		assert_eq!(parse("1024 KiB"), Some(MIB));
	}

	#[test]
	fn a_limit_without_a_unit_is_not_a_size() {
		assert_eq!(parse("-1"), None);
		assert_eq!(parse(""), None);
		assert_eq!(parse("12.4 parsecs"), None);
	}

	#[test]
	fn a_machine_that_will_not_say_runs_one_query_at_a_time() {
		assert_eq!(lane(LANE_MEMORY, 8), 1);
	}

	#[test]
	fn a_lane_is_worth_its_memory_and_never_outnumbers_the_threads() {
		assert_eq!(lane(9 * GIB, 8), 4);
		assert_eq!(lane(64 * GIB, 8), 8);
		assert_eq!(lane(GIB / 2, 8), 1);
	}

	#[test]
	fn the_dump_is_split_into_slices_of_a_size_a_query_can_digest() {
		assert_eq!(batch(191 * GIB, 1526), 48);
		assert_eq!(batch(CHUNK, 1526), 1);
	}

	#[test]
	fn a_slice_never_holds_less_than_a_shard() {
		assert_eq!(batch(191 * GIB, 4), 4);
		assert_eq!(batch(0, 0), 1);
	}

	#[test]
	fn the_memory_the_build_takes_leaves_the_machine_some() {
		let size = Size::of(12 * GIB, 191 * GIB, 1526);

		assert_eq!(size.memory, 9 * GIB);
		assert_eq!(size.limit(), "9216MiB".to_string());
	}
}
