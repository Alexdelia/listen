use std::io::{self, Write};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

pub fn line(label: &str) -> hmerr::Result<String> {
	print!("{B}{label}{D}: ");
	io::stdout().flush().map_err(|e| ioe!("stdout", e))?;

	let mut read = String::new();
	io::stdin()
		.read_line(&mut read)
		.map_err(|e| ioe!("stdin", e))?;

	let read = read.trim();
	if read.is_empty() {
		return Err(ge!(format!("{R}no {B}{label}{D} provided")).into());
	}

	Ok(read.to_string())
}

pub fn confirm(question: &str, default: bool) -> hmerr::Result<bool> {
	ux::ask_yn(question, default).map_err(|e| ioe!("stdin", e).into())
}
