use std::{
	fs::{self, File},
	io,
	path::{Path, PathBuf},
};

use indicatif::ProgressBar;
use miette::{IntoDiagnostic, WrapErr, miette};

use crate::bar;

const DIR: &str = "weights";
const REPOSITORY: &str = "https://huggingface.co/mtg-upf/discogs-maest-30s-pw-129e-519l/resolve";
const REVISION: &str = "6c35f32a350f74351870937d5ae0bae1d898d1df";
const PARTIAL: &str = "partial";

const FILE: [(&str, u64); 4] = [
	("config.json", 36_729),
	("preprocessor_config.json", 476),
	("feature_extraction_maest.py", 10_162),
	("model.safetensors", 347_827_260),
];

pub(super) fn ensure(model: &Path) -> miette::Result<PathBuf> {
	let dir = model.join(DIR);

	for (name, size) in FILE {
		let path = dir.join(name);
		if !present(&path, size) {
			fetch(name, &path, size)?;
		}
	}

	Ok(dir)
}

fn present(path: &Path, size: u64) -> bool {
	fs::metadata(path).is_ok_and(|metadata| metadata.len() == size)
}

fn fetch(name: &str, path: &Path, size: u64) -> miette::Result<()> {
	let url = format!("{REPOSITORY}/{REVISION}/{name}");
	let partial = path.with_extension(PARTIAL);
	listen_cache::prepare(&partial)?;

	let body = listen_agent::shared()
		.get(&url)
		.call()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to fetch {url}"))?
		.into_body()
		.into_reader();

	let bar = ProgressBar::new(size).with_style(bar::byte_template("weights", "cyan")?);
	let mut file = File::create(&partial)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot create {}", partial.display()))?;
	let written = io::copy(&mut bar.wrap_read(body), &mut file)
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to download {url}"))?;
	bar.finish_and_clear();

	if written != size {
		let _ = fs::remove_file(&partial);
		return Err(miette!(
			help = format!(
				"the model at {url} changed, update FILE in {file}",
				file = file!()
			),
			"{name} is {written} bytes, expected {size}"
		));
	}

	fs::rename(&partial, path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot move into {}", path.display()))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_weight_file_counts_as_present_only_at_its_exact_size() {
		let dir = std::env::temp_dir().join("declarative_listen_weights_present");
		let _ = fs::create_dir_all(&dir);
		let path = dir.join("config.json");
		fs::write(&path, b"1234").unwrap();

		assert!(present(&path, 4));
		assert!(!present(&path, 5));
		assert!(!present(&dir.join("missing.json"), 4));
		let _ = fs::remove_dir_all(&dir);
	}
}
