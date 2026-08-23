use std::{
	io::Read,
	process::{Child, Command, Stdio},
	thread,
};

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use indicatif::ProgressBar;

const READ: usize = 16 * 1024;

pub(crate) fn rsync(program: &str, arg: &[&str], bar: &ProgressBar) -> hmerr::Result<()> {
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

pub(crate) fn complaint(child: &mut Child) -> Option<thread::JoinHandle<String>> {
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
