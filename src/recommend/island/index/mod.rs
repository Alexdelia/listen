mod build;
mod dump;
mod open;
mod partial;
mod progress;

use std::path::Path;

pub(super) use open::{Index, Meta};

pub(super) fn ready() -> bool {
	open::dir().is_ok_and(|dir| open::built(&dir))
}

pub(super) fn ensure(declaration: &Path) -> hmerr::Result<Index> {
	let dir = open::dir()?;

	if !open::indexed(&dir) {
		let listen = dump::listen()?;
		build::run(&dir, &listen, declaration)?;
		dump::discard(&listen)?;
	}

	dump::artist_link(&dir.join(open::ARTIST_LINK))?;

	open::open(&dir)
}
