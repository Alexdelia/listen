use std::{
	hash::{DefaultHasher, Hash, Hasher},
	io::{BufRead, BufReader},
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

use ansi::abbrev::{B, D, G, R, Y};
use hmerr::{ge, ioe};
use indicatif::ProgressBar;
use listen_cache::text;

use crate::{bar, prompt};

use super::log::Log;

const NIX: &str = "nix";
const EXPRESSION: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/maest");
const SOURCE: [&str; 2] = [
	include_str!("../../../maest/default.nix"),
	include_str!("../../../maest/main.py"),
];
const RUNTIME: &str = "runtime";
const STAMP: &str = "runtime.hash";
const PROGRAM: &str = "bin/maest";
const NAMED: usize = 8;

pub(super) fn ensure(dir: &Path) -> hmerr::Result<Option<PathBuf>> {
	let runtime = dir.join(RUNTIME);
	let program = runtime.join(PROGRAM);
	let stamp = dir.join(STAMP);
	let fingerprint = fingerprint();

	if program.exists() && text::read(&stamp)?.as_deref() == Some(fingerprint.as_str()) {
		return Ok(Some(program));
	}

	let planned = dry_run()?;
	if planned.expected() > 0 {
		announce(&planned);
		if !prompt::confirm("build the tag suggestion model now", false)? {
			return Ok(None);
		}
	}

	listen_cache::prepare(&runtime)?;
	build(&runtime)?;
	text::write(&stamp, &fingerprint)?;

	Ok(Some(program))
}

fn fingerprint() -> String {
	let mut hasher = DefaultHasher::new();
	SOURCE.hash(&mut hasher);

	format!("{:016x}", hasher.finish())
}

fn nix(args: &[&str]) -> Command {
	let mut command = Command::new(NIX);
	command
		.args([
			"--extra-experimental-features",
			"nix-command",
			"build",
			"--file",
			EXPRESSION,
			"--log-format",
			"internal-json",
		])
		.args(args);

	command
}

fn dry_run() -> hmerr::Result<Log> {
	let output = nix(&["--dry-run", "--no-link"])
		.output()
		.map_err(|e| ioe!(format!("failed to execute {NIX}"), e))?;

	let mut log = Log::default();
	for line in String::from_utf8_lossy(&output.stderr).lines() {
		log.feed(line);
	}

	if !output.status.success() {
		return Err(failed("evaluate", &log));
	}

	Ok(log)
}

fn announce(planned: &Log) {
	println!(
		"{Y}the tag suggestion model needs {B}{built}{D}{Y} derivation(s) built and {B}{fetched}{D}{Y} path(s) fetched{D}",
		built = planned.build.len(),
		fetched = planned.fetch.len(),
	);

	let names = planned
		.build
		.iter()
		.chain(&planned.fetch)
		.collect::<Vec<_>>();
	for name in names.iter().take(NAMED) {
		println!("  {name}");
	}
	if names.len() > NAMED {
		println!("  {B}{more}{D} more", more = names.len() - NAMED);
	}

	println!(
		"{G}a cold build compiles NCCL for CUDA from source, about {B}75 min{D}{G} on 8 cores{D}"
	);
}

fn build(runtime: &Path) -> hmerr::Result<()> {
	let mut child = nix(&["--out-link", &runtime.to_string_lossy()])
		.stdout(Stdio::null())
		.stderr(Stdio::piped())
		.spawn()
		.map_err(|e| ioe!(format!("failed to execute {NIX}"), e))?;

	let stderr = child
		.stderr
		.take()
		.ok_or_else(|| ge!(format!("{R}no stderr from {B}{NIX}{D}")))?;

	let bar = bar::ticking(ProgressBar::new(0).with_style(bar::message_template("model", "cyan")?));
	let mut log = Log::default();

	for line in BufReader::new(stderr).lines() {
		let line = line.map_err(|e| ioe!(format!("{NIX} stderr"), e))?;
		log.feed(&line);

		bar.set_length(log.expected());
		bar.set_position(log.done());
		bar.set_message(log.current().unwrap_or_default().to_string());
	}

	let status = child
		.wait()
		.map_err(|e| ioe!(format!("failed to wait for {NIX}"), e))?;
	bar.finish_and_clear();

	if !status.success() {
		return Err(failed("build", &log));
	}

	Ok(())
}

fn failed(step: &str, log: &Log) -> Box<dyn std::error::Error> {
	ge!(
		format!(
			"{R}failed to {step} the tag suggestion model{D}\n{said}",
			said = log.said.join("\n")
		),
		h: format!("rerun by hand: {B}{G}{NIX} build --file {EXPRESSION}{D}")
	)
	.into()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_fingerprint_is_the_same_for_the_same_expression() {
		assert_eq!(fingerprint(), fingerprint());
		assert_eq!(fingerprint().len(), 16);
	}
}
