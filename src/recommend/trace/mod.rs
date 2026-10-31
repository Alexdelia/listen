mod artist;
mod collect;
mod document;

use miette::IntoDiagnostic;
use std::io::{self, Write};

pub(super) use artist::attach as attach_artist;
pub(super) use collect::collect;
pub(super) use document::{Arg, Document};

pub(super) fn write(document: &Document<'_>) -> miette::Result<()> {
	let mut stdout = io::stdout().lock();
	serde_json::to_writer_pretty(&mut stdout, document).into_diagnostic()?;
	writeln!(stdout).into_diagnostic()?;

	Ok(())
}
