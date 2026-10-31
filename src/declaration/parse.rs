use std::{collections::HashSet, fs, path::Path};

use miette::{Diagnostic, IntoDiagnostic, NamedSource, SourceSpan, WrapErr};

use super::Entry;

#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("{reason}")]
struct Malformed {
	reason: String,
	#[source_code]
	declaration: NamedSource<String>,
	#[label("{note}")]
	at: SourceSpan,
	note: &'static str,
}

pub(crate) fn parse<P>(path: P) -> miette::Result<Vec<Entry>>
where
	P: AsRef<Path>,
{
	let path = path.as_ref();

	let content = fs::read_to_string(path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;

	match ron::from_str::<Vec<Entry>>(&content) {
		Ok(list) => {
			duplicate(&list, &content, path)?;
			Ok(list)
		}
		Err(e) => {
			let at = offset(&content, e.span.start.line, e.span.start.col);

			Err(Malformed {
				reason: e.code.to_string(),
				declaration: named(path, content),
				at: (at, 1).into(),
				note: "here",
			})?
		}
	}
}

fn offset(content: &str, line: usize, col: usize) -> usize {
	let before: usize = content
		.split_inclusive('\n')
		.take(line.saturating_sub(1))
		.map(str::len)
		.sum();
	let within = content[before..]
		.char_indices()
		.nth(col.saturating_sub(1))
		.map_or(0, |(at, _)| at);

	before + within
}

fn named(path: &Path, content: String) -> NamedSource<String> {
	NamedSource::new(path.to_string_lossy(), content)
}

fn duplicate(list: &[Entry], content: &str, path: &Path) -> miette::Result<()> {
	let mut seen = HashSet::with_capacity(list.len());

	let Some(entry) = list.iter().find(|entry| !seen.insert(entry.s)) else {
		return Ok(());
	};

	let mbid = entry.s.to_string();
	let at = content
		.split_inclusive('\n')
		.scan(0, |start, line| {
			let at = *start;
			*start += line.len();
			Some((at, line))
		})
		.filter(|(_, line)| !line.trim_start().starts_with("//"))
		.filter_map(|(start, line)| {
			line.to_ascii_lowercase()
				.find(&mbid)
				.map(|within| start + within)
		})
		.nth(1)
		.unwrap_or_default();

	Err(Malformed {
		reason: format!("duplicate source {mbid}"),
		declaration: named(path, content.to_owned()),
		at: (at, mbid.len()).into(),
		note: "declared again here",
	})?
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use super::*;

	const FIRST: &str = "6678f68f-0e56-4105-a83c-50dc0361b73f";
	const SECOND: &str = "d3b47582-8395-4b55-b428-b64eca7761d0";

	fn written(name: &str, content: &str) -> PathBuf {
		let path = std::env::temp_dir().join(format!("declarative_listen_parse_{name}.ron"));
		let _ = fs::write(&path, content);

		path
	}

	fn refused(name: &str, content: &str) -> Option<miette::Report> {
		let path = written(name, content);
		let refused = parse(&path).err();
		let _ = fs::remove_file(&path);

		refused
	}

	fn labelled(refused: Option<&miette::Report>) -> Option<usize> {
		refused
			.and_then(|e| e.downcast_ref::<Malformed>())
			.map(|malformed| malformed.at.offset())
	}

	#[test]
	fn a_malformed_entry_is_labelled_where_ron_stopped() {
		let content = format!("[\n\t(s: \"{FIRST}\", q: 2, playlist: [] @),\n]\n");
		let said = refused("malformed", &content)
			.map(|e| format!("{e:?}"))
			.unwrap_or_default();

		assert!(
			said.contains("declarative_listen_parse_malformed.ron"),
			"{said}"
		);
		assert!(said.contains("playlist: [] @)"), "{said}");
	}

	#[test]
	fn a_malformed_entry_after_non_ascii_text_is_labelled_on_the_right_char() {
		let content = format!(
			"[\n\t// 小室哲哉 & のみこ\n\t(s: \"{FIRST}\", q: 2, playlist: [\"小室哲哉\"] @),\n]\n"
		);
		let refused = refused("non_ascii", &content);

		assert_eq!(labelled(refused.as_ref()), content.find('@'));
	}

	#[test]
	fn a_duplicate_is_labelled_on_its_second_declaration() {
		let content = format!(
			"[\n\t(s: \"{FIRST}\", q: 2, playlist: []),\n\t(s: \"{SECOND}\", q: 3, playlist: []),\n\t(s: \"{FIRST}\", q: 4, playlist: []),\n]\n"
		);
		let refused = refused("duplicate", &content);
		let said = refused
			.as_ref()
			.map(|e| format!("{e:?}"))
			.unwrap_or_default();

		assert!(said.contains("duplicate source"), "{said}");
		assert_eq!(labelled(refused.as_ref()), content.rfind(FIRST));
	}

	#[test]
	fn a_duplicate_first_seen_in_a_comment_or_another_case_labels_the_real_second_one() {
		let upper = FIRST.to_uppercase();
		let content = format!(
			"[\n\t// {upper} was moved\n\t(s: \"{FIRST}\", q: 2, playlist: []),\n\t(s: \"{upper}\", q: 4, playlist: []),\n]\n"
		);
		let refused = refused("duplicate_case", &content);

		assert_eq!(labelled(refused.as_ref()), content.rfind(&upper));
	}
}
