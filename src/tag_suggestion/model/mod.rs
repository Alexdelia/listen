mod build;
mod log;
mod run;
mod weights;

use std::{collections::HashMap, path::Path};

const DIR: &str = "maest";

pub(super) fn scores(audio: &Path) -> hmerr::Result<Option<HashMap<String, f32>>> {
	let dir = listen_cache::root()?.join(DIR);

	let Some(program) = build::ensure(&dir)? else {
		return Ok(None);
	};

	let weights = weights::ensure(&dir)?;

	run::scores(&program, audio, &weights).map(Some)
}
