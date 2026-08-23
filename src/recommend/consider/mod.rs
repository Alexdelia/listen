mod render;

use std::{ops::ControlFlow, path::Path};

use crate::{r#match, prompt};

use super::{declined, recommendation::Recommendation};

use render::render;

pub(super) async fn consider(
	path: &Path,
	index: usize,
	recommendation: &Recommendation,
) -> hmerr::Result<ControlFlow<()>> {
	let mbid = recommendation.mbid.to_string();
	println!("\n{}", render(index, recommendation));

	let declared = match r#match::run(path, &mbid, true).await {
		Ok(declared) => declared,
		Err(e) => {
			eprintln!("{e}");
			r#match::declare(path, &mbid)?
		}
	};

	if !declared {
		declined::add(recommendation.mbid)?;
	}

	println!();
	if prompt::confirm("continue", true)? {
		Ok(ControlFlow::Continue(()))
	} else {
		Ok(ControlFlow::Break(()))
	}
}
