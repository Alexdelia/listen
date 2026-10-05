mod artist;
mod collect;
mod document;

use std::io::{self, Write};

pub(super) use artist::attach as attach_artist;
pub(super) use collect::collect;
pub(super) use document::{Arg, Document};

pub(super) fn write(document: &Document<'_>) -> hmerr::Result<()> {
	let mut stdout = io::stdout().lock();
	serde_json::to_writer_pretty(&mut stdout, document)?;
	writeln!(stdout)?;

	Ok(())
}
