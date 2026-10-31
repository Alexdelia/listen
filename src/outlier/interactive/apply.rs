use std::{fs, path::Path};

use miette::{IntoDiagnostic, WrapErr, miette};
use ron::ser::PrettyConfig;

use crate::declaration::{Entry, Q, Source};

pub(super) fn set_q(path: &Path, mbid: Source, q: Q) -> miette::Result<()> {
	let content = fs::read_to_string(path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;

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
		return Err(miette!(
			"cannot set q: {mbid} not found in {path}",
			path = path.to_string_lossy()
		));
	}

	let mut updated = out.join("\n");
	if content.ends_with('\n') {
		updated.push('\n');
	}

	fs::write(path, updated)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot write {}", path.display()))?;

	Ok(())
}

fn rewrite(line: &str, mbid: Source, q: Q) -> miette::Result<Option<String>> {
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
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to serialize {mbid}"))?;

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
	use const_format::formatcp;

	use super::*;

	const ABC: &str = "abcabcab-abca-abca-abca-abcabcabcabc";
	const OTHER: &str = "99999999-9999-9999-9999-999999999999";

	fn id(mbid: &str) -> Source {
		mbid.parse().unwrap_or_default()
	}

	#[test]
	fn rewrite_pretty_entry() {
		assert_eq!(
			rewrite(
				formatcp!("\t(s: \"{ABC}\", q: 4, playlist: [\"charged\"]),"),
				id(ABC),
				1
			)
			.ok(),
			Some(Some(format!(
				"\t(s: \"{ABC}\", q: 1, playlist: [\"charged\"]),"
			)))
		);
	}

	#[test]
	fn rewrite_normalizes_odd_spacing() {
		assert_eq!(
			rewrite(formatcp!("\t(s:\"{ABC}\",q:2,playlist:[]),"), id(ABC), 3).ok(),
			Some(Some(format!("\t(s: \"{ABC}\", q: 3, playlist: []),")))
		);
	}

	#[test]
	fn other_mbid_is_left_alone() {
		assert_eq!(
			rewrite(
				formatcp!("\t(s: \"{ABC}\", q: 4, playlist: []),"),
				id(OTHER),
				1
			)
			.ok(),
			Some(None)
		);
	}

	#[test]
	fn comment_is_left_alone() {
		assert_eq!(
			rewrite(
				formatcp!("\t// (s: \"{ABC}\", q: 2, playlist: []),"),
				id(ABC),
				1
			)
			.ok(),
			Some(None)
		);
	}

	#[test]
	fn bracket_is_left_alone() {
		assert_eq!(rewrite("[", id(ABC), 1).ok(), Some(None));
	}
}
