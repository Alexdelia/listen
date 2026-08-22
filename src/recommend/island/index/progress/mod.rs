mod eta;

use std::{
	io::Read,
	process::{Child, Command, Stdio},
	sync::{
		Mutex, MutexGuard, PoisonError,
		atomic::{AtomicUsize, Ordering},
	},
	thread,
	time::Duration,
};

use ansi::abbrev::{B, D, F, R};
use hmerr::{ge, ioe};
use indicatif::{HumanBytes, MultiProgress, ProgressBar, ProgressStyle};

use eta::Eta;

const READ: usize = 16 * 1024;
const SPIN: Duration = Duration::from_millis(120);

const WIDTH: usize = 9;
const STEP: &str = "{pos:>4.bold.green}/{len:4.bold} {percent:>3.bold.green}%";
const WAITING_STEP: &str = "{pos:>4.dim}/{len:4.dim} {percent:>3.dim}%";
const BYTE: &str = "{bytes:>11.bold.green}/{total_bytes:11.bold} {bytes_per_sec:>12.bold.yellow}";
const WAITING_BYTE: &str = "{bytes:>11.dim}/{total_bytes:11.dim} {bytes_per_sec:>12.dim}";
const TIME: &str = "{elapsed:>5.bold.blue}|{eta:5.bold.magenta}";
const WAITING_TIME: &str = "    -|-    ";

#[derive(Clone, Copy)]
pub(super) enum Measure {
	Step(u64),
	Byte(u64),
}

impl Measure {
	fn total(self) -> u64 {
		match self {
			Self::Step(total) | Self::Byte(total) => total,
		}
	}

	fn field(self) -> &'static str {
		match self {
			Self::Step(_) => STEP,
			Self::Byte(_) => BYTE,
		}
	}

	fn waiting_field(self) -> &'static str {
		match self {
			Self::Step(_) => WAITING_STEP,
			Self::Byte(_) => WAITING_BYTE,
		}
	}
}

static BOARD: Mutex<Option<MultiProgress>> = Mutex::new(None);
static SHOWING: AtomicUsize = AtomicUsize::new(0);

fn held() -> MutexGuard<'static, Option<MultiProgress>> {
	BOARD.lock().unwrap_or_else(PoisonError::into_inner)
}

fn board() -> MultiProgress {
	held().get_or_insert_with(MultiProgress::new).clone()
}

fn showing() -> bool {
	SHOWING.load(Ordering::Relaxed) > 0
}

fn shown(bar: ProgressBar) -> ProgressBar {
	SHOWING.fetch_add(1, Ordering::Relaxed);

	board().add(bar)
}

pub(super) fn ended(bar: &ProgressBar) {
	if bar.is_finished() {
		return;
	}

	bar.disable_steady_tick();
	bar.finish();

	if SHOWING.fetch_sub(1, Ordering::Relaxed) == 1 {
		held().take();
	}
}

pub(super) fn say(line: impl AsRef<str>) {
	let line = line.as_ref();

	if !showing() {
		println!("{line}");
		return;
	}

	board().suspend(|| println!("{line}"));
}

pub(super) fn ask(question: &str, enter_is: bool) -> hmerr::Result<bool> {
	let answer = if showing() {
		board().suspend(|| ux::ask_yn(question, enter_is))
	} else {
		ux::ask_yn(question, enter_is)
	};

	answer.map_err(|e| ioe!("stdin", e).into())
}

pub(super) fn bytes(size: u64) -> String {
	HumanBytes(size).to_string()
}

pub(super) fn waiting_bar(measure: Measure, title: &str) -> hmerr::Result<ProgressBar> {
	let bar = shown(ProgressBar::new(measure.total()));
	bar.set_style(style(
		&format!("{F}{title}{D}", title = titled(title)),
		&format!("{field} {WAITING_TIME}", field = measure.waiting_field()),
		"white",
	)?);
	bar.tick();

	Ok(bar)
}

pub(super) fn started(bar: &ProgressBar, title: &str, measure: Measure) -> hmerr::Result<()> {
	let style = style(
		&titled(title),
		&format!("{field} {TIME}", field = measure.field()),
		"cyan",
	)?;

	bar.set_style(match measure {
		Measure::Step(_) => style.with_key(eta::KEY, Eta::new()),
		Measure::Byte(_) => style,
	});
	bar.reset_elapsed();
	bar.enable_steady_tick(SPIN);

	Ok(())
}

fn titled(title: &str) -> String {
	format!("{title:>WIDTH$}")
}

fn style(title: &str, field: &str, color: &str) -> hmerr::Result<ProgressStyle> {
	ProgressStyle::with_template(&format!("{title} {{wide_bar:.{color}/white}} {field}"))
		.map_err(|e| format!("failed to create progress style\n{e}").into())
}

pub(super) fn rsync(program: &str, arg: &[&str], bar: &ProgressBar) -> hmerr::Result<()> {
	let reached = bar.position();

	let mut child = Command::new(program)
		.args(arg)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.map_err(|e| ge!(format!("{R}failed to execute {B}{program}{D}\n{e}")))?;

	let complaint = complaint(&mut child);

	if let Some(out) = child.stdout.take() {
		follow(out, bar, reached);
	}

	let status = child
		.wait()
		.map_err(|e| ge!(format!("{R}failed to wait on {B}{program}{D}\n{e}")))?;

	let complaint = complaint.map_or_else(String::new, |read| read.join().unwrap_or_default());

	if !status.success() {
		return Err(ge!(
			format!("{R}{B}{program}{D}{R} failed{D}\n{complaint}"),
			h: "the transfer resumes where it stopped, run it again"
		)
		.into());
	}

	Ok(())
}

pub(super) fn complaint(child: &mut Child) -> Option<thread::JoinHandle<String>> {
	let mut err = child.stderr.take()?;

	Some(thread::spawn(move || {
		let mut said = String::new();
		let _ = err.read_to_string(&mut said);

		said.trim().to_string()
	}))
}

fn follow(mut out: impl Read, bar: &ProgressBar, reached: u64) {
	let mut buffer = [0u8; READ];
	let mut record = Vec::new();

	while let Ok(read) = out.read(&mut buffer) {
		if read == 0 {
			return;
		}

		for byte in &buffer[..read] {
			if *byte == b'\r' || *byte == b'\n' {
				if let Some(done) = transferred(&String::from_utf8_lossy(&record)) {
					bar.set_position(reached.saturating_add(done));
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
	fn a_transfer_carries_on_from_what_the_bar_already_reached() {
		let bar = ProgressBar::hidden();
		bar.set_length(1 << 20);
		bar.set_position(4096);

		follow(
			&b"  2,048  50%  1.00MB/s 0:00:01\n"[..],
			&bar,
			bar.position(),
		);

		assert_eq!(bar.position(), 4096 + 2048);
	}

	#[test]
	fn a_lone_transfer_starts_the_bar_where_it_stands() {
		let bar = ProgressBar::hidden();
		bar.set_length(1 << 20);

		follow(
			&b"  2,048  50%  1.00MB/s 0:00:01\n"[..],
			&bar,
			bar.position(),
		);

		assert_eq!(bar.position(), 2048);
	}

	#[test]
	fn a_waiting_stage_lines_up_with_a_running_one() {
		assert_eq!(WAITING_TIME.len(), "00:00|00:00".len());
	}

	#[test]
	fn what_a_program_complained_about_is_read_off_its_stderr() {
		let mut child = Command::new("sh")
			.args(["-c", "echo 'no such module' >&2"])
			.stderr(Stdio::piped())
			.spawn()
			.unwrap_or_else(|_| unreachable!());

		let complaint = complaint(&mut child).and_then(|read| read.join().ok());
		let _ = child.wait();

		assert_eq!(complaint, Some("no such module".to_string()));
	}

	#[test]
	fn a_program_that_said_nothing_complains_of_nothing() {
		let mut child = Command::new("true")
			.stderr(Stdio::piped())
			.spawn()
			.unwrap_or_else(|_| unreachable!());

		let complaint = complaint(&mut child).and_then(|read| read.join().ok());
		let _ = child.wait();

		assert_eq!(complaint, Some(String::new()));
	}
}
