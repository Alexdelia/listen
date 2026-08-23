use crate::prompt;

pub(crate) struct Terminal;

impl listen_index::Decide for Terminal {
	fn confirm(&self, question: &str, default: bool) -> hmerr::Result<bool> {
		prompt::confirm(question, default)
	}
}
