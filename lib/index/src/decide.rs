pub trait Decide {
	fn confirm(&self, question: &str, default: bool) -> hmerr::Result<bool>;
}

#[cfg(test)]
pub(crate) struct Refuse;

#[cfg(test)]
impl Decide for Refuse {
	fn confirm(&self, _question: &str, _default: bool) -> hmerr::Result<bool> {
		Ok(false)
	}
}
