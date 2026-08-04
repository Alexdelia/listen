use super::recommendation::Recommendation;

pub(super) trait Feed {
	fn next(&mut self) -> hmerr::Result<Option<Recommendation>>;
}
