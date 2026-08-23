use ansi::{
	DIM,
	abbrev::{B, D, Y},
};
use chrono::{DateTime, NaiveDateTime};

use listen_index::own::Gap;

use super::age;

const OFFSET: &str = "%Y-%m-%d %H:%M:%S%.f%:z";
const NAIVE: &str = "%Y-%m-%d %H:%M:%S%.f";

pub(super) struct Window {
	pub from: u64,
	pub to: u64,
}

pub(super) struct Covered {
	pub ago: u64,
	pub gap: Vec<Window>,
}

impl Covered {
	pub(super) fn days(&self, age: u64) -> u64 {
		age.saturating_sub(self.ago)
			.saturating_sub(self.missed(age))
	}

	fn missed(&self, age: u64) -> u64 {
		self.gap
			.iter()
			.map(|window| self.overlap(window, age))
			.sum()
	}

	fn overlap(&self, window: &Window, age: u64) -> u64 {
		window.from.min(age).saturating_sub(window.to.max(self.ago))
	}
}

impl Window {
	fn days(&self) -> u64 {
		self.from.saturating_sub(self.to)
	}
}

pub(super) fn covered(ago: u64, gap: &[Gap]) -> hmerr::Result<Covered> {
	let mut window = Vec::with_capacity(gap.len());
	let mut held = Vec::with_capacity(gap.len());

	for gap in gap {
		match read(gap)? {
			Some(read) => {
				held.push((gap, read.days()));
				window.push(read);
			}
			None => unreadable(gap),
		}
	}

	holed(&held);

	Ok(Covered { ago, gap: window })
}

fn holed(held: &[(&Gap, u64)]) {
	if held.is_empty() {
		return;
	}

	println!(
		"{Y}{B}{count}{D}{Y} window no dump covered, {B}{day}{D}{Y} day of listens out of the counts{D}",
		count = held.len(),
		day = held.iter().map(|(_, day)| day).sum::<u64>()
	);

	for (gap, day) in held {
		println!(
			"{DIM}{from}{D} {Y}to{D} {DIM}{to} ({day} day){D}",
			from = gap.from,
			to = gap.to
		);
	}
}

fn read(gap: &Gap) -> hmerr::Result<Option<Window>> {
	let Some((from, to)) = seconds(&gap.from).zip(seconds(&gap.to)) else {
		return Ok(None);
	};

	Ok(Some(Window {
		from: age::days_since(from)?,
		to: age::days_since(to)?,
	}))
}

pub(super) fn seconds(stamp: &str) -> Option<i64> {
	DateTime::parse_from_str(stamp, OFFSET)
		.map(|at| at.timestamp())
		.ok()
		.or_else(|| {
			NaiveDateTime::parse_from_str(stamp, NAIVE)
				.map(|at| at.and_utc().timestamp())
				.ok()
		})
}

fn unreadable(gap: &Gap) {
	eprintln!(
		"{Y}cannot read the window {B}{from}{D}{Y} to {B}{to}{D}{Y}, \
		the days it holds are counted as covered{D}",
		from = gap.from,
		to = gap.to
	);
}

#[cfg(test)]
mod tests {
	use super::*;

	fn window(from: u64, to: u64) -> Window {
		Window { from, to }
	}

	#[test]
	fn a_hole_inside_what_the_count_covers_is_not_days_the_entry_was_listened_over() {
		let covered = Covered {
			ago: 3,
			gap: vec![window(41, 30)],
		};

		assert_eq!(covered.days(100), 100 - 3 - 11);
	}

	#[test]
	fn a_hole_older_than_the_entry_never_shortens_it() {
		let covered = Covered {
			ago: 3,
			gap: vec![window(41, 30)],
		};

		assert_eq!(covered.days(20), 20 - 3);
	}

	#[test]
	fn an_entry_declared_inside_the_hole_only_loses_the_part_it_lived_through() {
		let covered = Covered {
			ago: 3,
			gap: vec![window(41, 30)],
		};

		assert_eq!(covered.days(35), 35 - 3 - 5);
	}

	#[test]
	fn a_hole_past_what_the_count_reaches_is_already_out_of_the_window() {
		let covered = Covered {
			ago: 30,
			gap: vec![window(20, 10)],
		};

		assert_eq!(covered.days(100), 100 - 30);
	}

	#[test]
	fn every_hole_together_is_what_the_count_never_saw() {
		let covered = Covered {
			ago: 3,
			gap: vec![window(41, 30), window(20, 18)],
		};

		assert_eq!(covered.days(100), 100 - 3 - 13);
	}

	#[test]
	fn a_timestamp_is_read_with_or_without_the_offset_it_carries() {
		assert_eq!(
			seconds("2026-07-12 00:00:04.001868+00:00"),
			Some(1_783_814_404)
		);
		assert_eq!(seconds("2026-07-12 00:00:04"), Some(1_783_814_404));
		assert_eq!(seconds("listen"), None);
	}

	#[test]
	fn a_window_that_cannot_be_read_leaves_the_others_standing() {
		let gap = [
			Gap {
				from: "listen".to_string(),
				to: "2026-07-23 00:00:03.690928+00:00".to_string(),
			},
			Gap {
				from: "2026-07-12 00:00:04.001868+00:00".to_string(),
				to: "2026-07-23 00:00:03.690928+00:00".to_string(),
			},
		];

		let covered = covered(0, &gap).unwrap_or_else(|_| unreachable!());

		assert_eq!(covered.gap.len(), 1);
		assert_eq!(covered.days(100), 100 - 11);
	}
}
