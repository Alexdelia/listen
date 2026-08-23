use std::sync::{
	Mutex, MutexGuard, PoisonError,
	atomic::{AtomicUsize, Ordering},
};

use hmerr::ioe;
use indicatif::{MultiProgress, ProgressBar};

static BOARD: Mutex<Option<MultiProgress>> = Mutex::new(None);
static SHOWING: AtomicUsize = AtomicUsize::new(0);

pub(super) fn shown(bar: ProgressBar) -> ProgressBar {
	SHOWING.fetch_add(1, Ordering::Relaxed);

	board().add(bar)
}

pub(crate) fn ended(bar: &ProgressBar) {
	if bar.is_finished() {
		return;
	}

	bar.disable_steady_tick();
	bar.finish();

	if SHOWING.fetch_sub(1, Ordering::Relaxed) == 1 {
		held().take();
	}
}

pub(crate) fn say(line: impl AsRef<str>) {
	let line = line.as_ref();

	if !showing() {
		println!("{line}");
		return;
	}

	board().suspend(|| println!("{line}"));
}

pub(crate) fn ask(question: &str, enter_is: bool) -> hmerr::Result<bool> {
	let answer = if showing() {
		board().suspend(|| ux::ask_yn(question, enter_is))
	} else {
		ux::ask_yn(question, enter_is)
	};

	answer.map_err(|e| ioe!("stdin", e).into())
}

fn held() -> MutexGuard<'static, Option<MultiProgress>> {
	BOARD.lock().unwrap_or_else(PoisonError::into_inner)
}

fn board() -> MultiProgress {
	held().get_or_insert_with(MultiProgress::new).clone()
}

fn showing() -> bool {
	SHOWING.load(Ordering::Relaxed) > 0
}
