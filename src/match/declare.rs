use std::{fmt, path::Path};

use crate::{
	declaration::{Q, Q_MAX},
	format::q_style,
	prompt,
};

use super::output;

#[derive(Debug, PartialEq, Eq)]
enum Answer {
	Declare(Option<Q>),
	Decline,
}

impl fmt::Display for Answer {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Declare(None) => write!(f, "{}", prompt::said(true)),
			Self::Declare(Some(q)) => write!(f, "{}", q_style(*q).apply_to(format!("q{q}"))),
			Self::Decline => write!(f, "{}", prompt::said(false)),
		}
	}
}

pub(super) fn run(path: &Path, mbid: &str, recommend: bool) -> miette::Result<bool> {
	let q = if recommend {
		match prompt::parsed(&format!("declare {}", hint()), answer)? {
			Answer::Declare(q) => q,
			Answer::Decline => return Ok(false),
		}
	} else {
		None
	};

	output::entry(path, mbid, q)?;

	Ok(true)
}

fn hint() -> String {
	let q = (0..=Q_MAX)
		.map(|q| q_style(q).apply_to(q).to_string())
		.collect::<Vec<_>>()
		.join("/");

	format!(
		"[{}/{}/{q}]",
		prompt::YES.apply_to("Y"),
		prompt::NO.apply_to("n")
	)
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
	fn answering_no_declines() {
		assert_eq!(answer("n"), Ok(Answer::Decline));
		assert_eq!(answer("No"), Ok(Answer::Decline));
	}

	#[test]
	fn an_answer_reads_back_as_what_it_declares() {
		let read = |answer: Answer| console::strip_ansi_codes(&answer.to_string()).into_owned();

		assert_eq!(read(Answer::Declare(None)), "yes");
		assert_eq!(read(Answer::Declare(Some(3))), "q3");
		assert_eq!(read(Answer::Decline), "no");
	}

	#[test]
	fn the_hint_offers_yes_no_and_every_q() {
		assert_eq!(console::strip_ansi_codes(&hint()), "[Y/n/0/1/2/3/4]");
	}

	#[test]
	fn a_q_out_of_range_or_anything_else_is_refused() {
		assert!(answer("5").is_err());
		assert!(answer("-1").is_err());
		assert!(answer("q3").is_err());
	}
}
