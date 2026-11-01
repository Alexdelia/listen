mod apply;
mod play;

use std::{ops::ControlFlow, path::Path};

use console::Style;

use crate::{
	declaration::Q,
	format,
	prompt::{self, Choice},
};

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

pub(super) fn run(analysis: &Analysis, path: &Path, username: &str) -> miette::Result<()> {
	render::matched(analysis);
	render::median(analysis);
	render::outlier_header(analysis.outlier.len());

	if analysis.outlier.is_empty() {
		println!("none");
	}

	let mut player = Player::new();
	for record in analysis.outlier.iter().filter(|record| record.listen > 0) {
		if review(record, path, &mut player)?.is_break() {
			return Ok(());
		}
	}

	render::undeclared(&analysis.undeclared, username);

	Ok(())
}

fn review(record: &Record, path: &Path, player: &mut Player) -> miette::Result<ControlFlow<()>> {
	loop {
		render::line(record);

		match ask(record)? {
			Answer::Quit => return Ok(ControlFlow::Break(())),
			Answer::Skip => return Ok(ControlFlow::Continue(())),
			Answer::Apply(q) => {
				apply::set_q(path, record.mbid, q)?;
				println!("{}", format::q_style(q).bold().apply_to(format!("q{q}")));
				return Ok(ControlFlow::Continue(()));
			}
			Answer::Play => {
				if let Err(e) = player.play(record.mbid) {
					eprintln!("{e:?}");
				}
			}
		}
	}
}

fn ask(record: &Record) -> miette::Result<Answer> {
	let answer = prompt::press("apply", &choices(record), 'y')?;

	Ok(match answer {
		'y' => Answer::Apply(record.observed),
		'p' => Answer::Play,
		'q' => Answer::Quit,
		digit @ '0'..='4' => Answer::Apply(digit as Q - b'0'),
		_ => Answer::Skip,
	})
}

fn choices(record: &Record) -> Vec<Choice> {
	let mut choices = Vec::with_capacity(9);

	choices.push(Choice {
		key: 'y',
		label: Some("apply"),
		style: format::q_style(record.observed).bold(),
		either_case: true,
	});
	choices.push(Choice {
		key: 'n',
		label: Some("skip"),
		style: Style::new(),
		either_case: true,
	});
	choices.push(Choice {
		key: 'p',
		label: Some("play"),
		style: Style::new().cyan(),
		either_case: true,
	});

	for q in 0..=TOP_Q {
		choices.push(Choice {
			key: char::from(b'0' + q),
			label: None,
			style: format::q_style(q),
			either_case: false,
		});
	}

	choices.push(Choice {
		key: 'q',
		label: Some("quit"),
		style: Style::new().red(),
		either_case: true,
	});

	choices
}
