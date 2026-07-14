mod apply;
mod play;

use std::{ops::ControlFlow, path::Path};

use ansi::abbrev::{B, CYA, D, R};
use hmerr::ioe;
use ux::AskKey;

use crate::{color, entry::Q};

use super::{
	analyze::{Analysis, Record},
	render,
};

use play::Player;

const TOP_Q: Q = 4;

enum Answer {
	Apply(Q),
	Skip,
	Play,
	Quit,
}

pub(super) fn run(analysis: &Analysis, path: &Path) -> hmerr::Result<()> {
	render::matched(analysis);
	render::median(analysis);
	render::outlier_header(analysis.outlier.len());

	if analysis.outlier.is_empty() {
		println!("none");
	}

	let mut player = Player::new();
	for record in &analysis.outlier {
		if review(record, path, &mut player)?.is_break() {
			return Ok(());
		}
	}

	render::undeclared(&analysis.undeclared);

	Ok(())
}

fn review(record: &Record, path: &Path, player: &mut Player) -> hmerr::Result<ControlFlow<()>> {
	loop {
		render::line(record);

		match ask(record)? {
			Answer::Quit => return Ok(ControlFlow::Break(())),
			Answer::Skip => return Ok(ControlFlow::Continue(())),
			Answer::Apply(q) => {
				apply::set_q(path, &record.mbid, q)?;
				println!("{B}{color}q{q}{D}", color = color::q(q));
				return Ok(ControlFlow::Continue(()));
			}
			Answer::Play => {
				if let Err(e) = player.play(&record.mbid) {
					eprintln!("{e}");
				}
			}
		}
	}
}

fn ask(record: &Record) -> hmerr::Result<Answer> {
	let answer = ux::ask("apply?", &key(record), Some('n')).map_err(|e| ioe!("stdin", e))?;

	Ok(match answer {
		'y' => Answer::Apply(record.observed),
		'p' => Answer::Play,
		'q' => Answer::Quit,
		digit @ '0'..='4' => Answer::Apply(digit as Q - b'0'),
		_ => Answer::Skip,
	})
}

fn key(record: &Record) -> Vec<AskKey> {
	let mut key = vec![
		AskKey::new('y', Some("apply"), true, Some(color::q(record.observed))),
		AskKey::new('n', Some("skip"), true, None::<&str>),
		AskKey::new('p', Some("play"), true, Some(CYA)),
		AskKey::new('q', Some("quit"), true, Some(R)),
	];

	for q in 0..=TOP_Q {
		key.push(AskKey::new(
			char::from(b'0' + q),
			None::<&str>,
			false,
			Some(color::q(q)),
		));
	}

	key
}
