use std::{fs, path::Path};

use miette::{IntoDiagnostic, WrapErr};

const COMMENT: &str = "//";
const Q_KEY: &str = "q:";
const Q_END: [char; 2] = [',', ')'];

pub(super) fn q(path: &Path, mbid: &str) -> miette::Result<Option<String>> {
	let content = fs::read_to_string(path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;

	Ok(q_in(&content, mbid).map(str::to_string))
}

fn q_in<'a>(content: &'a str, mbid: &str) -> Option<&'a str> {
	let quoted = format!("{:?}", mbid.to_ascii_lowercase());

	content
		.lines()
		.map(str::trim_start)
		.filter(|line| !line.starts_with(COMMENT))
		.find_map(|line| {
			let at = line.to_ascii_lowercase().find(&quoted)?;

			Some(written_q(&line[at + quoted.len()..]))
		})
}

fn written_q(after_mbid: &str) -> &str {
	after_mbid.split_once(Q_KEY).map_or("", |(_, q)| {
		q.split(Q_END).next().unwrap_or_default().trim()
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	const DECLARATION: &str = r#"[
	(s: "6678f68f-0e56-4105-a83c-50dc0361b73f", q: 2, playlist: ["charged"]),
	// (s: "43f9ffb6-05b6-4b66-b298-c3dc90e91eac", q: 1, playlist: []),
	(s: "d3b47582-8395-4b55-b428-b64eca7761d0", q: ?, playlist: []),
]"#;

	#[test]
	fn a_declared_mbid_reads_back_its_q() {
		assert_eq!(
			q_in(DECLARATION, "6678f68f-0e56-4105-a83c-50dc0361b73f"),
			Some("2")
		);
	}

	#[test]
	fn an_entry_still_waiting_for_its_q_is_declared() {
		assert_eq!(
			q_in(DECLARATION, "d3b47582-8395-4b55-b428-b64eca7761d0"),
			Some("?")
		);
	}

	#[test]
	fn a_commented_out_entry_is_not_declared() {
		assert_eq!(
			q_in(DECLARATION, "43f9ffb6-05b6-4b66-b298-c3dc90e91eac"),
			None
		);
	}

	#[test]
	fn an_mbid_is_found_whatever_its_case() {
		assert_eq!(
			q_in(DECLARATION, "6678F68F-0E56-4105-A83C-50DC0361B73F"),
			Some("2")
		);
	}

	#[test]
	fn an_unknown_mbid_is_not_declared() {
		assert_eq!(
			q_in(DECLARATION, "b830bfe2-17eb-4826-8ec8-0b5fe62f8768"),
			None
		);
	}
}
