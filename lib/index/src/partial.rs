use std::{fs, path::Path};

use miette::{IntoDiagnostic, WrapErr};

use super::keep;

const EXT: &str = "writing";

pub(super) fn write(
	into: &Path,
	produce: impl FnOnce(&Path) -> miette::Result<()>,
) -> miette::Result<()> {
	let partial = into.with_extension(EXT);

	if let Err(e) = produce(&partial) {
		let _ = keep::discard(&partial);
		return Err(e);
	}

	fs::rename(&partial, into)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot move into {}", into.display()))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use miette::miette;

	use super::*;

	#[test]
	fn what_was_produced_lands_under_the_final_name() {
		let dir = crate::scratch::of("partial", "landed");
		let into = dir.join("out.parquet");

		let done = write(&into, |partial| {
			fs::write(partial, b"content")
				.into_diagnostic()
				.wrap_err("cannot write partial")?;

			Ok(())
		});

		assert!(done.is_ok());
		assert_eq!(fs::read(&into).unwrap_or_default(), b"content");
		assert!(!into.with_extension(EXT).exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_half_written_file_never_reaches_the_final_name() {
		let dir = crate::scratch::of("partial", "failed");
		let into = dir.join("out.parquet");

		let done = write(&into, |partial| {
			fs::write(partial, b"half")
				.into_diagnostic()
				.wrap_err("cannot write partial")?;

			Err(miette!("interrupted"))
		});

		assert!(done.is_err());
		assert!(!into.exists());
		assert!(!into.with_extension(EXT).exists());
		let _ = fs::remove_dir_all(&dir);
	}
}
