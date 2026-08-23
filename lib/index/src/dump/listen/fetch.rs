use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, F, R, Y};
use hmerr::{ge, ioe};
use indicatif::ProgressBar;

use super::{
	super::{
		super::{board::Board, keep, progress},
		rsync, space,
		stage::{self, Stage},
	},
	LISTEN, Listen, Offer,
	find::{name_of, timestamp},
	published::{MODULE, newest},
};

const EXT: &str = ".tar";

pub(crate) fn fetch(root: &Path, offer: &Offer) -> hmerr::Result<Option<Listen>> {
	let dump =
		newest()?.ok_or_else(|| ge!(format!("{R}nothing published under {B}{MODULE}{D}")))?;

	fetch_named(root, &dump, offer)
}

pub(crate) fn fetch_named(root: &Path, dump: &str, offer: &Offer) -> hmerr::Result<Option<Listen>> {
	let url = format!("{host}/{MODULE}/{dump}/", host = rsync::HOST);
	let archive = rsync::biggest(&url, EXT)?;
	let tar = root.join(&archive.name);

	progress::say(format!(
		"\n{F}listen dump {B}{dump}{D}{F}: {B}{Y}{size}{D}{F}, {B}{Y}+{size}{D}{F} unpacked, \
		deleted once index built{D}",
		size = progress::bytes(archive.size)
	));

	progress::say(format!("{F}{reason}{D}", reason = offer.reason));

	if !progress::ask("download", offer.enter_is)? {
		return Ok(None);
	}

	space::require(root, space::unpacking(&tar, archive.size))?;

	let board = Board::of(&stage::LISTEN, &archive.size)?;

	board.run(Stage::Download, |bar| {
		rsync::pull(&format!("{url}{name}", name = archive.name), &tar, bar)
	})?;
	board.run(Stage::Verify, |_| {
		rsync::checked(&url, root, &rsync::checksum(&archive.name))
	})?;

	let dir = board.run(Stage::Unpack, |bar| unpack(&tar, root, bar))?;
	keep::discard(&tar)?;

	Ok(Some(Listen {
		name: name_of(&dir),
		dir,
	}))
}

fn unpack(tar: &Path, root: &Path, bar: &ProgressBar) -> hmerr::Result<PathBuf> {
	let file = fs::File::open(tar).map_err(|e| ioe!(tar.to_string_lossy(), e))?;

	tar::Archive::new(bar.wrap_read(file))
		.unpack(root)
		.map_err(|e| ioe!(tar.to_string_lossy(), e))?;

	let dir = root.join(LISTEN);
	let inner = fs::read_dir(root)
		.map_err(|e| ioe!(root.to_string_lossy(), e))?
		.filter_map(Result::ok)
		.map(|entry| entry.path())
		.filter(|path| path.is_dir() && *path != dir)
		.filter_map(|path| Some((timestamp(&path)?, path)))
		.max()
		.map(|(_, path)| path)
		.ok_or_else(|| ge!(format!("{R}the listen archive held no dump directory{D}")))?;

	if dir.exists() {
		fs::remove_dir_all(&dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;
	}
	fs::rename(&inner, &dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;

	Ok(dir)
}
