use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

const COUNT: &str = "{pos:>4.bold.green}/{len:4.bold}";
const BYTES: &str = "{bytes:>4.bold.green}/{total_bytes:4.bold}";
const TIME: &str = "{elapsed:>3.bold.blue}|{eta:3.bold.magenta}";

const TICK: Duration = Duration::from_millis(200);

pub(crate) fn template(title: &str, color: &str) -> hmerr::Result<ProgressStyle> {
	style(title, color, COUNT, "")
}

pub(crate) fn byte_template(title: &str, color: &str) -> hmerr::Result<ProgressStyle> {
	style(title, color, BYTES, "")
}

pub(crate) fn message_template(title: &str, color: &str) -> hmerr::Result<ProgressStyle> {
	style(title, color, COUNT, " {msg}")
}

pub(crate) fn spinner(title: &str, color: &str) -> hmerr::Result<ProgressBar> {
	let style = ProgressStyle::with_template(&format!(
		"{title:>8} {{spinner:.{color}}} {{elapsed:>3.bold.blue}} {{msg}}"
	))
	.map_err(|e| format!("failed to create progress style\n{e}"))?;

	let bar = ProgressBar::new_spinner().with_style(style);
	bar.enable_steady_tick(TICK);

	Ok(bar)
}

pub(crate) fn ticking(bar: ProgressBar) -> ProgressBar {
	bar.enable_steady_tick(TICK);

	bar
}

fn style(title: &str, color: &str, count: &str, tail: &str) -> hmerr::Result<ProgressStyle> {
	let title = format!("{title:>8}");

	ProgressStyle::with_template(
		&[
			&title,
			" {wide_bar:.",
			color,
			"/white} ",
			count,
			" {percent:>3.bold.green}% ",
			TIME,
			tail,
		]
		.join(""),
	)
	.map_err(|e| format!("failed to create progress style\n{e}").into())
}
