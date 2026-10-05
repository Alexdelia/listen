use std::{collections::HashSet, path::Path};

use serde::Serialize;

use crate::declaration::Source;

use super::{declared, declined};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Reason {
	Declared,
	Declined,
	Duplicate,
	Listened,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Count {
	pub declared: usize,
	pub declined: usize,
}

#[derive(Default)]
pub(super) struct Skip {
	declared: HashSet<Source>,
	declined: HashSet<Source>,
	shown: HashSet<Source>,
}

impl Skip {
	pub(super) fn load(path: &Path) -> hmerr::Result<Self> {
		Ok(Self::new(declared::sources(path)?, declined::load()?))
	}

	pub(super) fn new(declared: HashSet<Source>, declined: HashSet<Source>) -> Self {
		Self {
			declared,
			declined,
			shown: HashSet::new(),
		}
	}

	pub(super) fn stale(&self, mbid: Source) -> Option<Reason> {
		if self.declared.contains(&mbid) {
			return Some(Reason::Declared);
		}

		if self.declined.contains(&mbid) {
			return Some(Reason::Declined);
		}

		self.shown.contains(&mbid).then_some(Reason::Duplicate)
	}

	pub(super) fn fresh(&mut self, mbid: Source) -> bool {
		self.stale(mbid).is_none() && self.shown.insert(mbid)
	}

	pub(super) fn seen(&self, mbid: Source) -> bool {
		self.stale(mbid).is_some()
	}

	pub(super) fn count(&self) -> Count {
		Count {
			declared: self.declared.len(),
			declined: self.declined.len(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn mbid(byte: u8) -> Source {
		Source::from_bytes([byte; 16])
	}

	fn skip() -> Skip {
		Skip::new(
			[mbid(1), mbid(3)].into_iter().collect(),
			[mbid(2), mbid(3)].into_iter().collect(),
		)
	}

	#[test]
	fn a_declared_recording_is_stale_as_declared() {
		assert_eq!(skip().stale(mbid(1)), Some(Reason::Declared));
	}

	#[test]
	fn a_declined_recording_is_stale_as_declined() {
		assert_eq!(skip().stale(mbid(2)), Some(Reason::Declined));
	}

	#[test]
	fn a_recording_both_declared_and_declined_is_stale_as_declared() {
		assert_eq!(skip().stale(mbid(3)), Some(Reason::Declared));
	}

	#[test]
	fn a_recording_shown_once_is_a_duplicate_the_second_time() {
		let mut skip = skip();

		assert!(skip.fresh(mbid(9)));
		assert_eq!(skip.stale(mbid(9)), Some(Reason::Duplicate));
		assert!(!skip.fresh(mbid(9)));
	}

	#[test]
	fn the_count_is_what_was_loaded_not_what_was_shown() {
		let mut skip = skip();
		skip.fresh(mbid(9));

		assert_eq!(
			skip.count(),
			Count {
				declared: 2,
				declined: 2
			}
		);
	}
}
