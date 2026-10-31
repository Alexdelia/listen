use std::{
	fs,
	path::{Path, PathBuf},
};

use console::style;
use indicatif::ProgressBar;
use miette::{IntoDiagnostic, WrapErr, miette};

use super::{
	super::{
		super::{board::Board, decide::Decide, keep, progress},
		rsync, space,
		stage::{self, Stage},
	},
	LISTEN, Listen, Offer,
	find::{name_of, timestamp},
	published::{MODULE, newest},
};

const EXT: &str = ".tar";

pub(crate) fn fetch(
	root: &Path,
	offer: &Offer,
	decide: &dyn Decide,
) -> miette::Result<Option<Listen>> {
	let dump = newest()?.ok_or_else(|| miette!("nothing published under {MODULE}"))?;

	fetch_named(root, &dump, offer, decide)
}

pub(crate) fn fetch_named(
	root: &Path,
	dump: &str,
	offer: &Offer,
	decide: &dyn Decide,
) -> miette::Result<Option<Listen>> {
	let url = format!("{host}/{MODULE}/{dump}/", host = rsync::HOST);
	let archive = rsync::biggest(&url, EXT)?;
	let tar = root.join(&archive.name);

	progress::say(format!(
		"\n{}{}{}{}{}{}{}",
		style("listen dump ").dim(),
		style(dump).dim().bold(),
		style(": ").dim(),
		style(progress::bytes(archive.size)).dim().bold().yellow(),
		style(", ").dim(),
		style(format!("+{size}", size = progress::bytes(archive.size)))
			.dim()
			.bold()
			.yellow(),
		style(
			" unpacked, \
		deleted once index built"
		)
		.dim()
	));

	progress::say(style(offer.reason).dim().to_string());

	if !progress::confirm(decide, "download", offer.default)? {
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

fn unpack(tar: &Path, root: &Path, bar: &ProgressBar) -> miette::Result<PathBuf> {
	let file = fs::File::open(tar)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot open {}", tar.display()))?;

	tar::Archive::new(bar.wrap_read(file))
		.unpack(root)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot unpack {}", tar.display()))?;

	let dir = root.join(LISTEN);
	let inner = fs::read_dir(root)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot list {}", root.display()))?
		.filter_map(Result::ok)
		.map(|entry| entry.path())
		.filter(|path| path.is_dir() && *path != dir)
		.filter_map(|path| Some((timestamp(&path)?, path)))
		.max()
		.map(|(_, path)| path)
		.ok_or_else(|| miette!("the listen archive held no dump directory"))?;

	if dir.exists() {
		fs::remove_dir_all(&dir)
			.into_diagnostic()
			.wrap_err_with(|| format!("cannot remove {}", dir.display()))?;
	}
	fs::rename(&inner, &dir)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot move into {}", dir.display()))?;

	Ok(dir)
}
