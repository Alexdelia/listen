use std::{
	fs, io,
	path::{Path, PathBuf},
	process::{Command, ExitStatus, Stdio},
};

use console::style;
use indicatif::ProgressBar;
use miette::{IntoDiagnostic, WrapErr, miette};

use super::{
	super::{board::Board, decide::Decide, keep, partial, progress},
	rsync, space,
	stage::{self, Stage},
};

const MODULE: &str = "data/fullexport";
const ARCHIVE: &str = "mbdump.tar.bz2";
const SUMS: &str = "SHA256SUMS";
const TAR: &str = "tar";

const TABLE: [&str; 2] = ["artist", "l_artist_artist"];

const ARTIST_COLUMN: &str = "['id','gid']";
const LINK_COLUMN: &str = "['id','link','entity0','entity1']";
const READ: &str = r"delim='\t', header=false, quote='', escape='', nullstr='\N', all_varchar=true";

pub(super) fn build(root: &Path, link: &Path, decide: &dyn Decide) -> miette::Result<()> {
	let dump = rsync::latest_marker(MODULE, root)?;
	let url = format!("{host}/{MODULE}/{dump}/", host = rsync::HOST);
	let archive = rsync::list(&url)?
		.into_iter()
		.find(|entry| entry.name == ARCHIVE)
		.ok_or_else(|| miette!("no {ARCHIVE} inside {dump}"))?;

	space::require(root, space::unpacking(&root.join(ARCHIVE), archive.size))?;

	progress::say(format!(
		"\n{}{}{}{}{}",
		style("artist relations: musicbrainz dump ").dim(),
		style(dump).dim().bold(),
		style(", ").dim(),
		style(progress::bytes(archive.size)).dim().bold().yellow(),
		style(
			", \
		2 tables kept, archive deleted after"
		)
		.dim()
	));

	if !progress::confirm(decide, "download", true)? {
		return Err(refused());
	}

	let board = Board::of(&stage::MUSIC_BRAINZ, &archive.size)?;

	board.run(Stage::Download, |bar| {
		rsync::pull(&format!("{url}{ARCHIVE}"), &root.join(ARCHIVE), bar)
	})?;
	board.run(Stage::Verify, |_| rsync::checked(&url, root, SUMS))?;

	let table = board.run(Stage::Unpack, |bar| unpack(root, bar))?;
	board.run(Stage::Relation, |_| load(&table, link))?;

	keep::discard(&root.join(ARCHIVE))?;
	keep::discard(&table)?;

	Ok(())
}

fn unpack(root: &Path, bar: &ProgressBar) -> miette::Result<PathBuf> {
	let archive = root.join(ARCHIVE);
	let into = root.join("mb");
	fs::create_dir_all(&into)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot create {}", into.display()))?;

	let mut argument = vec![
		"--extract".to_string(),
		"--bzip2".to_string(),
		"--file".to_string(),
		"-".to_string(),
		"--directory".to_string(),
		into.to_string_lossy().to_string(),
		"--strip-components=1".to_string(),
		"--wildcards".to_string(),
	];
	argument.extend(TABLE.iter().map(|table| format!("*/{table}")));

	let mut child = Command::new(TAR)
		.args(&argument)
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::piped())
		.spawn()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to execute {TAR}"))?;

	let complaint = progress::complaint(&mut child);

	let file = fs::File::open(&archive)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot open {}", archive.display()))?;
	let fed = child.stdin.take().map_or(Ok(0), |mut sink| {
		io::copy(&mut bar.wrap_read(file), &mut sink)
	});

	let status = child
		.wait()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to wait on {TAR}"))?;

	let complaint = complaint.map_or_else(String::new, |read| read.join().unwrap_or_default());

	unpacked(status, fed, &complaint)?;

	for table in TABLE {
		if !into.join(table).exists() {
			return Err(miette!("{table} is not in {ARCHIVE}"));
		}
	}

	Ok(into)
}

fn unpacked(status: ExitStatus, fed: io::Result<u64>, complaint: &str) -> miette::Result<()> {
	if !status.success() {
		return Err(miette!("{TAR} could not unpack {ARCHIVE}\n{complaint}"));
	}

	fed.into_diagnostic()
		.wrap_err_with(|| format!("cannot feed {ARCHIVE} to {TAR}"))?;

	Ok(())
}

fn load(table: &Path, link: &Path) -> miette::Result<()> {
	let db = duckdb::Connection::open_in_memory().into_diagnostic()?;

	partial::write(link, |link| {
		db.execute_batch(&format!(
			r"
copy (
	with artist as (
		select id, gid from read_csv('{table}/artist', {READ}, names={ARTIST_COLUMN})
	),
	link as (
		select entity0, entity1 from read_csv('{table}/l_artist_artist', {READ}, names={LINK_COLUMN})
	),
	edge as (
		select a.gid::uuid as artist_mbid, b.gid::uuid as related_mbid
		from link l
		join artist a on a.id = l.entity0
		join artist b on b.id = l.entity1
	)
	select artist_mbid, related_mbid from edge
	union
	select related_mbid, artist_mbid from edge
) to '{link}' (format parquet, compression zstd);
",
			table = table.display(),
			link = link.display()
		))
		.into_diagnostic()?;

		Ok(())
	})
}

fn refused() -> miette::Report {
	miette!(
		help = "artist relations decide which artists count as already known, they are required",
		"cancelled"
	)
}

#[cfg(test)]
mod tests {
	use std::os::unix::process::ExitStatusExt;

	use super::*;

	fn exit(code: i32) -> ExitStatus {
		ExitStatus::from_raw(code << 8)
	}

	fn broken_pipe() -> io::Result<u64> {
		Err(io::Error::from(io::ErrorKind::BrokenPipe))
	}

	fn message(done: miette::Result<()>) -> String {
		done.err().map(|e| format!("{e:?}")).unwrap_or_default()
	}

	#[test]
	fn a_tar_that_died_is_what_broke_the_pipe() {
		let said = message(unpacked(exit(2), broken_pipe(), "tar: unexpected eof"));

		assert!(said.contains("could not unpack"), "{said}");
	}

	#[test]
	fn a_pipe_that_broke_on_its_own_is_still_reported() {
		let said = message(unpacked(exit(0), broken_pipe(), ""));

		assert!(said.contains("broken pipe"), "{said}");
	}

	#[test]
	fn a_whole_archive_fed_to_a_happy_tar_is_unpacked() {
		assert!(unpacked(exit(0), Ok(1 << 20), "").is_ok());
	}
}
