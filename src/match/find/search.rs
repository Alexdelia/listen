use std::process::Command;

use miette::{IntoDiagnostic, WrapErr, miette};

use crate::streaming_source::{StreamingSource, YT_DLP};

use super::push_unique;

// opaque music.youtube.com url param selecting the "Songs" tab (never "Videos")
const SONGS_FILTER: &str = "EgWKAQIIAWoKEAkQBRAKEAMQBA%3D%3D";

const RESULT_PER_QUERY: usize = 4;
const MAX_CANDIDATE: usize = 8;

pub(super) fn search(query: &[String]) -> miette::Result<Vec<String>> {
	let mut id = Vec::new();

	for q in query {
		for found in search_one(q)? {
			push_unique(&mut id, found);
		}

		if id.len() >= MAX_CANDIDATE {
			break;
		}
	}

	id.truncate(MAX_CANDIDATE);
	Ok(id)
}

fn search_one(query: &str) -> miette::Result<Vec<String>> {
	let url = format!(
		"{base_url}/search?q={q}&sp={SONGS_FILTER}",
		base_url = StreamingSource::YouTubeMusic.base_url(),
		q = percent_encode(query)
	);
	let end = RESULT_PER_QUERY.to_string();

	let output = Command::new(YT_DLP)
		.args([
			"--flat-playlist",
			"--no-warnings",
			"--playlist-end",
			&end,
			"--print",
			"%(id)s",
			&url,
		])
		.output()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to execute {YT_DLP}"))?;

	if !output.status.success() {
		return Err(miette!(
			"{YT_DLP} failed to search {query}\n{e}",
			e = String::from_utf8_lossy(&output.stderr)
		));
	}

	Ok(String::from_utf8_lossy(&output.stdout)
		.lines()
		.map(str::trim)
		.filter(|id| !id.is_empty())
		.map(String::from)
		.collect())
}

const HEX_DIGIT: &[u8; 16] = b"0123456789ABCDEF";

fn percent_encode(s: &str) -> String {
	let mut out = String::with_capacity(s.len() * 3);

	for b in s.bytes() {
		match b {
			b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
				out.push(b as char);
			}
			_ => {
				out.push('%');
				out.push(HEX_DIGIT[(b >> 4) as usize] as char);
				out.push(HEX_DIGIT[(b & 0x0f) as usize] as char);
			}
		}
	}

	out
}
