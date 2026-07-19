use std::fmt::Write;

use ansi::abbrev::{B, D, R};

use super::matching::Mismatch;

const MAX_MISS: usize = 3;

pub(super) struct Miss {
	pub(super) url: String,
	pub(super) reason: Mismatch,
}

pub(super) fn block(mut miss: Vec<Miss>) -> String {
	miss.sort_by_key(|m| m.reason.rank());
	miss.truncate(MAX_MISS);

	let mut out = String::new();
	for m in &miss {
		let _ = write!(
			out,
			"\n{url} {reason}",
			url = m.url,
			reason = reason(&m.reason)
		);
	}

	out
}

fn reason(mismatch: &Mismatch) -> String {
	match mismatch {
		Mismatch::Duration(delta) => format!("{B}{R}{delta:+}s{D}"),
		Mismatch::NoDuration => format!("{B}{R}no duration{D}"),
		Mismatch::Title => format!("{B}{R}title{D}"),
		Mismatch::NoTitle => format!("{B}{R}no title{D}"),
		Mismatch::NotSong(category) => {
			format!(
				"{B}{R}{found_category}{D}",
				found_category = category.as_deref().unwrap_or("not a song")
			)
		}
	}
}
