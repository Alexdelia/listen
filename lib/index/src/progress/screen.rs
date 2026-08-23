use std::sync::{
	Mutex, MutexGuard, PoisonError,
	atomic::{AtomicUsize, Ordering},
};

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

	suspended(|| println!("{line}"));
}

pub(super) fn suspended<T>(work: impl FnOnce() -> T) -> T {
	if !showing() {
		return work();
	}

	board().suspend(work)
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
