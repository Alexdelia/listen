use std::thread;

use ansi::abbrev::{B, D, R};
use hmerr::ge;

pub(super) fn both<A: Send, B: Send>(
	a: impl FnOnce() -> hmerr::Result<A> + Send,
	b: impl FnOnce() -> hmerr::Result<B> + Send,
) -> hmerr::Result<(A, B)> {
	thread::scope(|scope| {
		let aside = scope.spawn(|| a().map_err(|e| e.to_string()));
		let here = joined(Ok(b().map_err(|e| e.to_string())))?;

		Ok((joined(aside.join())?, here))
	})
}

pub(super) fn all<A: Send, B: Send, C: Send>(
	a: impl FnOnce() -> hmerr::Result<A> + Send,
	b: impl FnOnce() -> hmerr::Result<B> + Send,
	c: impl FnOnce() -> hmerr::Result<C> + Send,
) -> hmerr::Result<(A, B, C)> {
	let (a, (b, c)) = both(a, || both(b, c))?;

	Ok((a, b, c))
}

pub(super) fn joined<T>(done: thread::Result<Result<T, String>>) -> hmerr::Result<T> {
	match done {
		Ok(Ok(value)) => Ok(value),
		Ok(Err(e)) => Err(ge!(e).into()),
		Err(_) => Err(ge!(
			format!("{R}a stage {B}died{D}"),
			h: "what it had already written is kept, run again to resume from it"
		)
		.into()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn fail() -> hmerr::Result<()> {
		Err(ge!("stage failed".to_string()).into())
	}

	#[test]
	fn what_two_stages_produce_comes_back_in_the_order_they_were_given() {
		assert_eq!(both(|| Ok(1), || Ok("two")).unwrap_or_default(), (1, "two"));
	}

	#[test]
	fn three_stages_run_together_and_keep_their_order() {
		assert_eq!(
			all(|| Ok(1), || Ok("two"), || Ok(3.0)).unwrap_or_default(),
			(1, "two", 3.0)
		);
	}

	#[test]
	fn a_stage_that_fails_fails_the_whole_run() {
		assert!(both(fail, || Ok(())).is_err());
		assert!(both(|| Ok(()), fail).is_err());
		assert!(all(|| Ok(()), fail, || Ok(())).is_err());
	}

	#[test]
	fn what_a_failed_stage_said_is_what_comes_back() {
		let said = both(fail, || Ok(()))
			.err()
			.map(|e| format!("{e}"))
			.unwrap_or_default();

		assert!(said.contains("stage failed"), "{said}");
	}
}
