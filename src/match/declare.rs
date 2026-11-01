use std::path::Path;

use crate::{
	declaration::{Q, Q_MAX},
	prompt,
};

use super::output;

#[derive(Debug, PartialEq, Eq)]
enum Answer {
	Declare(Option<Q>),
	Decline,
}

pub(super) fn run(path: &Path, mbid: &str, recommend: bool) -> miette::Result<bool> {
	let q = if recommend {
		match prompt::parsed(&format!("declare [Y/n/0-{Q_MAX}]"), answer)? {
			Answer::Declare(q) => q,
			Answer::Decline => return Ok(false),
		}
	} else {
		None
	};

	output::entry(path, mbid, q)?;

	Ok(true)
}

fn answer(input: &str) -> Result<Answer, String> {
	match input.to_ascii_lowercase().as_str() {
		"" | "y" | "yes" => return Ok(Answer::Declare(None)),
		"n" | "no" => return Ok(Answer::Decline),
		_ => {}
	}

	input
		.parse::<Q>()
		.ok()
		.filter(|q| *q <= Q_MAX)
		.map(|q| Answer::Declare(Some(q)))
		.ok_or_else(|| format!("expected y, n or a q in 0..={Q_MAX}, not '{input}'"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn enter_alone_declares_with_an_unknown_q() {
		assert_eq!(answer(""), Ok(Answer::Declare(None)));
		assert_eq!(answer("Y"), Ok(Answer::Declare(None)));
	}

	#[test]
	fn a_typed_q_declares_with_it() {
		assert_eq!(answer("0"), Ok(Answer::Declare(Some(0))));
		assert_eq!(answer("4"), Ok(Answer::Declare(Some(Q_MAX))));
	}

	#[test]
	fn no_declines() {
		assert_eq!(answer("n"), Ok(Answer::Decline));
		assert_eq!(answer("No"), Ok(Answer::Decline));
	}

	#[test]
	fn a_q_out_of_range_or_anything_else_is_refused() {
		assert!(answer("5").is_err());
		assert!(answer("-1").is_err());
		assert!(answer("q3").is_err());
	}
}
