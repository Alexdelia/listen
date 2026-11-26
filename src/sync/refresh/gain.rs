use std::sync::{Mutex, PoisonError};

use indicatif::ProgressBar;

use crate::{bar, declaration::Entry, library};

use super::super::gain;

pub(crate) fn run(list: &[Entry]) -> miette::Result<()> {
	let existing = list
		.iter()
		.map(|entry| library::recording::path(entry.s))
		.filter(|path| path.exists())
		.collect::<Vec<_>>();

	let pb = ProgressBar::new(existing.len() as u64);
	pb.set_style(bar::template("gain", "white")?);
	pb.tick();

	let err = Mutex::new(vec![]);

	gain::each(existing, |status| {
		if let Err(e) = status {
			err.lock().unwrap_or_else(PoisonError::into_inner).push(e);
		}
		pb.inc(1);
	});

	pb.finish();

	super::said(&err.into_inner().unwrap_or_else(PoisonError::into_inner));

	Ok(())
}
