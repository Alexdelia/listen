use hmerr::ioe;

pub(crate) struct Terminal;

impl listen_index::Decide for Terminal {
	fn confirm(&self, question: &str, default: bool) -> hmerr::Result<bool> {
		ux::ask_yn(question, default).map_err(|e| ioe!("stdin", e).into())
	}
}
