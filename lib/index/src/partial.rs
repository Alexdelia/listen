use std::{fs, path::Path};

use hmerr::ioe;

use super::keep;

const EXT: &str = "writing";

pub(super) fn write(
	into: &Path,
	produce: impl FnOnce(&Path) -> hmerr::Result<()>,
) -> hmerr::Result<()> {
	let partial = into.with_extension(EXT);

	if let Err(e) = produce(&partial) {
		let _ = keep::discard(&partial);
		return Err(e);
	}

	fs::rename(&partial, into).map_err(|e| ioe!(into.to_string_lossy(), e))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use hmerr::ge;

	use super::*;

	fn dir(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_partial_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	#[test]
	fn what_was_produced_lands_under_the_final_name() {
		let dir = dir("landed");
		let into = dir.join("out.parquet");

		let done = write(&into, |partial| {
			fs::write(partial, b"content").map_err(|e| ioe!("partial", e))?;

			Ok(())
		});

		assert!(done.is_ok());
		assert_eq!(fs::read(&into).unwrap_or_default(), b"content");
		assert!(!into.with_extension(EXT).exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_half_written_file_never_reaches_the_final_name() {
		let dir = dir("failed");
		let into = dir.join("out.parquet");

		let done = write(&into, |partial| {
			fs::write(partial, b"half").map_err(|e| ioe!("partial", e))?;

			Err(ge!("interrupted".to_string()).into())
		});

		assert!(done.is_err());
		assert!(!into.exists());
		assert!(!into.with_extension(EXT).exists());
		let _ = fs::remove_dir_all(&dir);
	}
}
