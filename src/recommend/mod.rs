mod fetch;

use std::{collections::HashSet, ops::ControlFlow, path::Path};

use ansi::abbrev::{B, CYA, D, Y};
use hmerr::ioe;

use crate::{cache, declaration::Source, r#match};

use fetch::Recommendation;

pub async fn run(path: &Path, username: Option<&str>, unlistened: bool) -> hmerr::Result<()> {
	let username = cache::username::resolve(username)?;

	let mut declared = declared(path)?;

	let mut offset = 0;
	loop {
		let page = fetch::page(&username, offset)?;
		if page.recommendation.is_empty() {
			break;
		}

		for recommendation in &page.recommendation {
			if consider(path, recommendation, unlistened, &mut declared)
				.await?
				.is_break()
			{
				return Ok(());
			}
		}

		offset += page.fetched;
		if page.fetched == 0 || offset >= page.total {
			break;
		}
	}

	Ok(())
}

async fn consider(
	path: &Path,
	recommendation: &Recommendation,
	unlistened: bool,
	declared: &mut HashSet<Source>,
) -> hmerr::Result<ControlFlow<()>> {
	if unlistened && recommendation.latest_listened_at.is_some() {
		return Ok(ControlFlow::Continue(()));
	}

	if !declared.insert(recommendation.mbid) {
		return Ok(ControlFlow::Continue(()));
	}

	let mbid = recommendation.mbid.to_string();
	println!(
		"\n{B}{mbid}{D} {Y}{score:.3}{D}{last}",
		score = recommendation.score,
		last = recommendation
			.latest_listened_at
			.as_deref()
			.map(|at| format!(" {CYA}{at}{D}"))
			.unwrap_or_default(),
	);

	match r#match::run(path, &mbid, r#match::Prompt::Review).await {
		Ok(flow) => return Ok(flow),
		Err(e) => {
			eprintln!("{e}");
			if !ux::ask_yn("no match, continue", true).map_err(|e| ioe!("stdin", e))? {
				return Ok(ControlFlow::Break(()));
			}
		}
	}

	Ok(ControlFlow::Continue(()))
}

fn declared(path: &Path) -> hmerr::Result<HashSet<Source>> {
	Ok(crate::declaration::parse::parse(path)?
		.into_iter()
		.map(|entry| entry.s)
		.collect())
}
