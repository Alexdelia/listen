use std::process::Command;

use hmerr::ioe;

pub(super) fn run(command: &mut Command, url: &str) -> hmerr::Result<()> {
	match command.output() {
		Ok(output) => {
			if output.status.success() {
				return Ok(());
			}

			Err(format!(
				"failed to download {url}\n{e}",
				e = String::from_utf8_lossy(&output.stderr)
			)
			.into())
		}
		Err(e) => Err(ioe!(
			format!(
				"failed to execute {downloader}",
				downloader = command.get_program().to_string_lossy()
			),
			e,
		)
		.into()),
	}
}
