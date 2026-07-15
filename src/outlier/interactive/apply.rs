use std::{fs, path::Path};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};
use ron::ser::PrettyConfig;

use crate::declaration::{Entry, Q};

pub(super) fn set_q(path: &Path, mbid: &str, q: Q) -> hmerr::Result<()> {
	let content = fs::read_to_string(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	let mut out = Vec::new();
	let mut set = false;

	for line in content.lines() {
		match if set { None } else { rewrite(line, mbid, q)? } {
			Some(rewritten) => {
				out.push(rewritten);
				set = true;
			}
			None => out.push(line.to_string()),
		}
	}

	if !set {
		return Err(ge!(format!(
			"{R}cannot set q: {B}{mbid}{D} not found in {B}{path}{D}",
			path = path.to_string_lossy(),
		))
		.into());
	}

	let mut updated = out.join("\n");
	if content.ends_with('\n') {
		updated.push('\n');
	}

	fs::write(path, updated).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn rewrite(line: &str, mbid: &str, q: Q) -> hmerr::Result<Option<String>> {
	let trimmed = line.trim();
	let payload = trimmed.strip_suffix(',').unwrap_or(trimmed);

	let Ok(mut entry) = ron::from_str::<Entry>(payload) else {
		return Ok(None);
	};

	if entry.s != mbid {
		return Ok(None);
	}

	entry.q = q;

	let indent = &line[..line.len() - line.trim_start().len()];
	let body = ron::ser::to_string_pretty(&entry, config())
		.map_err(|e| ge!(format!("{R}failed to serialize {B}{mbid}{D}\n{e}")))?;

	Ok(Some(format!("{indent}{body},")))
}

fn config() -> PrettyConfig {
	PrettyConfig::new()
		.struct_names(false)
		.compact_structs(true)
		.compact_arrays(true)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rewrite_pretty_entry() {
		assert_eq!(
			rewrite("\t(s: \"abc\", q: 4, playlist: [\"charged\"]),", "abc", 1).ok(),
			Some(Some(
				"\t(s: \"abc\", q: 1, playlist: [\"charged\"]),".to_string()
			))
		);
	}

	#[test]
	fn rewrite_normalizes_odd_spacing() {
		assert_eq!(
			rewrite("\t(s:\"abc\",q:2,playlist:[]),", "abc", 3).ok(),
			Some(Some("\t(s: \"abc\", q: 3, playlist: []),".to_string()))
		);
	}

	#[test]
	fn other_mbid_is_left_alone() {
		assert_eq!(
			rewrite("\t(s: \"abc\", q: 4, playlist: []),", "xyz", 1).ok(),
			Some(None)
		);
	}

	#[test]
	fn comment_is_left_alone() {
		assert_eq!(
			rewrite("\t// (s: \"abc\", q: 2, playlist: []),", "abc", 1).ok(),
			Some(None)
		);
	}

	#[test]
	fn bracket_is_left_alone() {
		assert_eq!(rewrite("[", "abc", 1).ok(), Some(None));
	}
}
