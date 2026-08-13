use super::{recommendation::Recommendation, skip::Skip};

pub(super) trait Feed {
	fn next(&mut self, skip: &Skip) -> hmerr::Result<Option<Recommendation>>;
}
