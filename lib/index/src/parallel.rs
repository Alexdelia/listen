use std::thread;

use miette::miette;

pub(super) fn both<A: Send, B: Send>(
	a: impl FnOnce() -> miette::Result<A> + Send,
	b: impl FnOnce() -> miette::Result<B> + Send,
) -> miette::Result<(A, B)> {
	thread::scope(|scope| {
		let aside = scope.spawn(a);
		let here = b()?;

		Ok((joined(aside.join())?, here))
	})
}

pub(super) fn all<A: Send, B: Send, C: Send>(
	a: impl FnOnce() -> miette::Result<A> + Send,
	b: impl FnOnce() -> miette::Result<B> + Send,
	c: impl FnOnce() -> miette::Result<C> + Send,
) -> miette::Result<(A, B, C)> {
	let (a, (b, c)) = both(a, || both(b, c))?;

	Ok((a, b, c))
}

pub(super) fn joined<T>(done: thread::Result<miette::Result<T>>) -> miette::Result<T> {
	done.unwrap_or_else(|_| {
		Err(miette!(
			help = "what it had already written is kept, run again to resume from it",
			"a stage died"
		))
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn fail() -> miette::Result<()> {
		Err(miette!("stage failed"))
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
			.map(|e| format!("{e:?}"))
			.unwrap_or_default();

		assert!(said.contains("stage failed"), "{said}");
	}
}
