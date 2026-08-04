use std::{collections::HashSet, path::Path};

use crate::declaration::Source;

use super::{declared, declined};

#[derive(Default)]
pub(super) struct Skip(HashSet<Source>);

impl Skip {
	pub(super) fn load(path: &Path) -> hmerr::Result<Self> {
		let mut source = declared::sources(path)?;
		source.extend(declined::load()?);

		Ok(Self(source))
	}

	pub(super) fn fresh(&mut self, mbid: Source) -> bool {
		self.0.insert(mbid)
	}
}
