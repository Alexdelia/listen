use std::{
	io::Read,
	process::{Command, Stdio},
	sync::OnceLock,
	time::Duration,
};

use ansi::abbrev::{B, D, F, R};
use hmerr::ge;
use indicatif::{HumanBytes, MultiProgress, ProgressBar, ProgressStyle};

const READ: usize = 16 * 1024;
const SPIN: Duration = Duration::from_millis(120);

const WIDTH: usize = 9;
const STEP: &str = "{pos:>4.bold.green}/{len:4.bold} {percent:>3.bold.green}%";
const WAITING_STEP: &str = "{pos:>4.dim}/{len:4.dim} {percent:>3.dim}%";
const TIME: &str = "{elapsed:>5.bold.blue}|{eta:5.bold.magenta}";
const WAITING_TIME: &str = "    -|-    ";

static BOARD: OnceLock<MultiProgress> = OnceLock::new();

fn board() -> &'static MultiProgress {
	BOARD.get_or_init(MultiProgress::new)
}

pub(super) fn say(line: impl AsRef<str>) {
	board().suspend(|| println!("{}", line.as_ref()));
}

pub(super) fn bytes(size: u64) -> String {
	HumanBytes(size).to_string()
}

pub(super) fn byte_bar(total: u64, title: &str) -> hmerr::Result<ProgressBar> {
	let bar = board().add(ProgressBar::new(total));
	bar.set_style(style(
		&titled(title),
		&format!(
			"{{bytes:>11.bold.green}}/{{total_bytes:11.bold}} \
			{{bytes_per_sec:>12.bold.yellow}} {TIME}"
		),
		"cyan",
	)?);

	Ok(bar)
}

pub(super) fn waiting_bar(total: u64, title: &str) -> hmerr::Result<ProgressBar> {
	let bar = board().add(ProgressBar::new(total));
	bar.set_style(waiting(title)?);
	bar.tick();

	Ok(bar)
}

pub(super) fn started(bar: &ProgressBar, title: &str) -> hmerr::Result<()> {
	bar.set_style(style(&titled(title), &format!("{STEP} {TIME}"), "cyan")?);
	bar.reset_elapsed();
	bar.enable_steady_tick(SPIN);

	Ok(())
}

pub(super) fn ended(bar: &ProgressBar) {
	bar.disable_steady_tick();
	bar.finish();
}

fn waiting(title: &str) -> hmerr::Result<ProgressStyle> {
	style(
		&format!("{F}{title}{D}", title = titled(title)),
		&format!("{WAITING_STEP} {WAITING_TIME}"),
		"white",
	)
}

fn titled(title: &str) -> String {
	format!("{title:>WIDTH$}")
}

fn style(title: &str, field: &str, color: &str) -> hmerr::Result<ProgressStyle> {
	ProgressStyle::with_template(&format!("{title} {{wide_bar:.{color}/white}} {field}"))
		.map_err(|e| format!("failed to create progress style\n{e}").into())
}

pub(super) fn rsync(program: &str, arg: &[&str], total: u64) -> hmerr::Result<()> {
	let bar = byte_bar(total, "download")?;

	let mut child = Command::new(program)
		.args(arg)
		.stdout(Stdio::piped())
		.spawn()
		.map_err(|e| ge!(format!("{R}failed to execute {B}{program}{D}\n{e}")))?;

	if let Some(out) = child.stdout.take() {
		follow(out, &bar);
	}

	let status = child
		.wait()
		.map_err(|e| ge!(format!("{R}failed to wait on {B}{program}{D}\n{e}")))?;

	bar.finish();

	if !status.success() {
		return Err(ge!(
			format!("{R}{B}{program}{D}{R} failed{D}"),
			h: "the transfer resumes where it stopped, run it again"
		)
		.into());
	}

	Ok(())
}

fn follow(mut out: impl Read, bar: &ProgressBar) {
	let mut buffer = [0u8; READ];
	let mut record = Vec::new();

	while let Ok(read) = out.read(&mut buffer) {
		if read == 0 {
			return;
		}

		for byte in &buffer[..read] {
			if *byte == b'\r' || *byte == b'\n' {
				if let Some(done) = transferred(&String::from_utf8_lossy(&record)) {
					bar.set_position(done);
				}
				record.clear();
			} else {
				record.push(*byte);
			}
		}
	}
}

fn transferred(record: &str) -> Option<u64> {
	let mut field = record.split_whitespace();
	let done = field.next()?.replace(',', "").parse().ok()?;

	if !field.next()?.ends_with('%') {
		return None;
	}

	Some(done)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_progress_record_reports_the_bytes_transferred() {
		assert_eq!(
			transferred("  1,234,567,890  12%   45.67MB/s    0:01:23"),
			Some(1_234_567_890)
		);
	}

	#[test]
	fn a_progress_record_without_separators_still_parses() {
		assert_eq!(transferred("204800 100% 1.00MB/s 0:00:01"), Some(204_800));
	}

	#[test]
	fn a_file_name_line_is_not_progress() {
		assert_eq!(transferred("listenbrainz-spark-dump-2593-full.tar"), None);
	}

	#[test]
	fn the_rsync_preamble_is_not_progress() {
		assert_eq!(transferred("sending incremental file list"), None);
	}

	#[test]
	fn an_empty_record_is_not_progress() {
		assert_eq!(transferred(""), None);
		assert_eq!(transferred("   "), None);
	}

	#[test]
	fn a_size_without_a_percentage_is_not_progress() {
		assert_eq!(transferred("1,234 something else"), None);
	}

	#[test]
	fn a_waiting_stage_lines_up_with_a_running_one() {
		assert_eq!(WAITING_TIME.len(), "00:00|00:00".len());
	}
}
