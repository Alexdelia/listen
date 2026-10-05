use ansi::abbrev::{B, D, F};

use crate::args::RecommendSort;

use super::Report;

pub(super) fn header(report: &Report) -> String {
	let scope = match report.sort {
		RecommendSort::Popularity => format!(
			"{ranked} listened of {total} recording",
			ranked = report.ranked,
			total = report.recording
		),
		RecommendSort::Newest => format!(
			"{dated} dated of {total} recording",
			dated = report.dated,
			total = report.recording
		),
	};

	format!("{B}{artist}{D} {F}{scope}{D}", artist = report.artist)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn report(sort: RecommendSort, recording: usize, dated: usize, ranked: usize) -> Report {
		Report {
			artist: "Mili".to_string(),
			sort,
			recording,
			dated,
			ranked,
		}
	}

	#[test]
	fn popularity_counts_what_it_kept_out_of_the_catalogue() {
		let shown = header(&report(RecommendSort::Popularity, 20, 17, 14));

		assert!(shown.contains("Mili"), "{shown}");
		assert!(shown.contains("14 listened of 20 recording"), "{shown}");
	}

	#[test]
	fn newest_counts_the_dated_recording_it_can_order() {
		let shown = header(&report(RecommendSort::Newest, 20, 17, 20));

		assert!(shown.contains("17 dated of 20 recording"), "{shown}");
	}
}
