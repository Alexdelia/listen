use std::fmt::Write;

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
		Mismatch::Duration(delta) => format!("{delta:+}s"),
		Mismatch::NoDuration => "no duration".to_owned(),
		Mismatch::Title => "title".to_owned(),
		Mismatch::NoTitle => "no title".to_owned(),
		Mismatch::NotSong(category) => category.as_deref().unwrap_or("not a song").to_owned(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_miss_reason_reads_as_plain_text_inside_the_error() {
		assert_eq!(reason(&Mismatch::Duration(-2)), "-2s");
		assert_eq!(reason(&Mismatch::Title), "title");
		assert_eq!(reason(&Mismatch::NotSong(None)), "not a song");
	}
}
