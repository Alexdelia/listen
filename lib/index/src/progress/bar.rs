use std::time::Duration;

use ansi::abbrev::{D, F};
use indicatif::{ProgressBar, ProgressStyle};

use super::{
	eta::{self, Eta},
	screen::shown,
};

const SPIN: Duration = Duration::from_millis(120);

const WIDTH: usize = 9;
const STEP: &str = "{pos:>4.bold.green}/{len:4.bold} {percent:>3.bold.green}%";
const WAITING_STEP: &str = "{pos:>4.dim}/{len:4.dim} {percent:>3.dim}%";
const BYTE: &str = "{bytes:>11.bold.green}/{total_bytes:11.bold} {bytes_per_sec:>12.bold.yellow}";
const WAITING_BYTE: &str = "{bytes:>11.dim}/{total_bytes:11.dim} {bytes_per_sec:>12.dim}";
const TIME: &str = "{elapsed:>5.bold.blue}|{eta:5.bold.magenta}";
const WAITING_TIME: &str = "    -|-    ";

#[derive(Clone, Copy)]
pub(crate) enum Measure {
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

pub(crate) fn waiting_bar(measure: Measure, title: &str) -> hmerr::Result<ProgressBar> {
	let bar = shown(ProgressBar::new(measure.total()));
	bar.set_style(style(
		&format!("{F}{title}{D}", title = titled(title)),
		&format!("{field} {WAITING_TIME}", field = measure.waiting_field()),
		"white",
	)?);
	bar.tick();

	Ok(bar)
}

pub(crate) fn started(bar: &ProgressBar, title: &str, measure: Measure) -> hmerr::Result<()> {
	let style = style(
		&titled(title),
		&format!("{field} {TIME}", field = measure.field()),
		"cyan",
	)?;

	bar.set_style(style.with_key(eta::KEY, Eta::new()));
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_waiting_stage_lines_up_with_a_running_one() {
		assert_eq!(WAITING_TIME.len(), "00:00|00:00".len());
	}
}
