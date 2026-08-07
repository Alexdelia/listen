use ansi::abbrev::{B, D, F};

use crate::args::RecommendSort;

use super::catalogue::Catalogue;

pub(super) fn header(sort: RecommendSort, catalogue: &Catalogue, ranked: usize) -> String {
	let total = catalogue.recording.len();

	let scope = match sort {
		RecommendSort::Popularity => format!("{ranked} listened of {total} recording"),
		RecommendSort::Newest => format!(
			"{dated} dated of {total} recording",
			dated = catalogue.released.len()
		),
	};

	format!("{B}{artist}{D} {F}{scope}{D}", artist = catalogue.artist)
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use chrono::NaiveDate;

	use super::*;
	use crate::declaration::Source;

	fn catalogue(total: u8, dated: u8) -> Catalogue {
		Catalogue {
			artist: "Mili".to_string(),
			recording: (0..total).map(|n| Source::from_bytes([n; 16])).collect(),
			released: (0..dated)
				.filter_map(|n| {
					Some((
						Source::from_bytes([n; 16]),
						NaiveDate::from_ymd_opt(2026, 8, 3)?,
					))
				})
				.collect::<HashMap<_, _>>(),
		}
	}

	#[test]
	fn popularity_counts_what_it_kept_out_of_the_catalogue() {
		let shown = header(RecommendSort::Popularity, &catalogue(20, 17), 14);

		assert!(shown.contains("Mili"), "{shown}");
		assert!(shown.contains("14 listened of 20 recording"), "{shown}");
	}

	#[test]
	fn newest_counts_the_dated_recording_it_can_order() {
		let shown = header(RecommendSort::Newest, &catalogue(20, 17), 20);

		assert!(shown.contains("17 dated of 20 recording"), "{shown}");
	}
}
