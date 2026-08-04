mod render;

use std::{ops::ControlFlow, path::Path};

use hmerr::ioe;

use crate::r#match;

use super::{declined, recommendation::Recommendation};

use render::render;

pub(super) async fn consider(
	path: &Path,
	index: usize,
	recommendation: &Recommendation,
) -> hmerr::Result<ControlFlow<()>> {
	let mbid = recommendation.mbid.to_string();
	println!("\n{}", render(index, recommendation));

	match r#match::run(path, &mbid, true).await {
		Ok(true) => {}
		Ok(false) => declined::add(recommendation.mbid)?,
		Err(e) => eprintln!("{e}"),
	}

	println!();
	if ux::ask_yn("continue", true).map_err(|e| ioe!("stdin", e))? {
		Ok(ControlFlow::Continue(()))
	} else {
		Ok(ControlFlow::Break(()))
	}
}
