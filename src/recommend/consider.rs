use std::{collections::HashSet, ops::ControlFlow, path::Path};

use ansi::abbrev::{B, CYA, D, Y};
use chrono::{DateTime, Months, Utc};
use hmerr::ioe;

use crate::{declaration::Source, r#match};

use super::fetch::Recommendation;

const DATE_FORMAT: &str = "%Y-%m-%d";
const TIME_FORMAT: &str = "%H:%M";

pub(super) async fn consider(
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
			.map(|at| format!(" {CYA}{at}{D}", at = listened(at)))
			.unwrap_or_default(),
	);

	if let Err(e) = r#match::run(path, &mbid).await {
		eprintln!("{e}");
	}

	println!();
	if ux::ask_yn("continue", true).map_err(|e| ioe!("stdin", e))? {
		Ok(ControlFlow::Continue(()))
	} else {
		Ok(ControlFlow::Break(()))
	}
}

fn listened(at: DateTime<Utc>) -> String {
	let recent = Utc::now()
		.checked_sub_months(Months::new(1))
		.is_some_and(|cutoff| at >= cutoff);

	let date_str = at.format(DATE_FORMAT).to_string();

	if recent {
		let time_str = at.format(TIME_FORMAT).to_string();
		format!("{date_str} {time_str}")
	} else {
		date_str
	}
}
