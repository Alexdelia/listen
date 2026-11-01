use std::fmt::Display;

use console::{Key, Style, Term, strip_ansi_codes, style};
use dialoguer::{Confirm, Input, MultiSelect, theme::ColorfulTheme};
use miette::{IntoDiagnostic, WrapErr, bail, miette};

pub struct Choice {
	pub key: char,
	pub label: Option<&'static str>,
	pub style: Style,
	pub either_case: bool,
}

pub fn line(label: &str) -> miette::Result<String> {
	Input::<String>::with_theme(&ColorfulTheme::default())
		.with_prompt(label)
		.validate_with(|input: &String| typed(input).map(drop))
		.interact_text()
		.map(|input| input.trim().to_owned())
		.into_diagnostic()
		.wrap_err("cannot read stdin")
}

pub fn parsed<T, E: Display>(
	label: &str,
	parse: impl Fn(&str) -> Result<T, E>,
) -> miette::Result<T> {
	let input = Input::<String>::with_theme(&ColorfulTheme::default())
		.with_prompt(label)
		.allow_empty(true)
		.validate_with(|input: &String| parse(input.trim()).map(drop).map_err(|e| e.to_string()))
		.interact_text()
		.into_diagnostic()
		.wrap_err("cannot read stdin")?;

	parse(input.trim()).map_err(|e| miette!("{e}"))
}

fn typed(input: &str) -> Result<&str, &'static str> {
	let input = input.trim();
	if input.is_empty() {
		return Err("nothing typed");
	}

	Ok(input)
}

pub fn confirm(asked: &str, default: bool) -> miette::Result<bool> {
	Confirm::with_theme(&ColorfulTheme::default())
		.with_prompt(asked)
		.default(default)
		.interact()
		.into_diagnostic()
		.wrap_err("cannot read stdin")
}

#[must_use]
pub fn interactive() -> bool {
	Term::stderr().is_term()
}

pub fn check(asked: &str, items: &[String]) -> miette::Result<Vec<usize>> {
	checked(&Term::stderr(), asked, items)
}

fn checked(term: &Term, asked: &str, items: &[String]) -> miette::Result<Vec<usize>> {
	if !term.is_term() {
		bail!(
			help = "run it from a terminal, stderr is where the question is put",
			"no terminal to read {asked} from"
		);
	}

	MultiSelect::with_theme(&ColorfulTheme::default())
		.with_prompt(asked)
		.items(items.iter().map(|item| strip_ansi_codes(item)))
		.report(false)
		.interact_on_opt(term)
		.map(Option::unwrap_or_default)
		.into_diagnostic()
		.wrap_err("cannot read stdin")
}

pub fn press(asked: &str, choices: &[Choice], on_enter: char) -> miette::Result<char> {
	pressed(&Term::stderr(), asked, choices, on_enter)
}

fn pressed(term: &Term, asked: &str, choices: &[Choice], on_enter: char) -> miette::Result<char> {
	if !term.is_term() {
		bail!(
			help = "run it from a terminal, stderr is where the question is put",
			"no terminal to read {asked} from"
		);
	}

	loop {
		term.write_str(&question(asked, choices))
			.into_diagnostic()
			.wrap_err("cannot write stderr")?;

		let pressed = term
			.read_key()
			.into_diagnostic()
			.wrap_err("cannot read stdin")?;

		let chosen = resolve(choices, on_enter, &pressed);
		let said = chosen.map_or_else(|| waiting(choices, &pressed), |key| key.to_string());
		term.write_line(&said)
			.into_diagnostic()
			.wrap_err("cannot write stderr")?;

		if let Some(key) = chosen {
			return Ok(key);
		}
	}
}

fn resolve(choices: &[Choice], on_enter: char, pressed: &Key) -> Option<char> {
	match pressed {
		Key::Enter => Some(on_enter),
		Key::Char(pressed) => choices
			.iter()
			.find(|choice| {
				choice.key == *pressed
					|| (choice.either_case && choice.key.eq_ignore_ascii_case(pressed))
			})
			.map(|choice| choice.key),
		_ => None,
	}
}

fn question(asked: &str, choices: &[Choice]) -> String {
	let keys = choices
		.iter()
		.map(|choice| choice.style.apply_to(choice.key).to_string())
		.collect::<Vec<_>>()
		.join("/");

	let theme = ColorfulTheme::default();

	format!(
		"{} {} [{keys}] ",
		theme.prompt_prefix,
		theme.prompt_style.apply_to(asked)
	)
}

fn waiting(choices: &[Choice], pressed: &Key) -> String {
	let expected = choices
		.iter()
		.map(|choice| {
			let key = choice.style.apply_to(choice.key);
			choice
				.label
				.map_or_else(|| key.to_string(), |label| format!("{key} ({label})"))
		})
		.collect::<Vec<_>>()
		.join(", ");
	let pressed = match pressed {
		Key::Char(pressed) => pressed.to_string(),
		other => format!("{other:?}"),
	};

	format!("waiting for {expected}, not '{}'", style(pressed).bold())
}

#[cfg(test)]
mod tests {
	use std::fs::File;

	use console::{Key, Style, strip_ansi_codes};

	use super::*;

	fn no_terminal() -> Term {
		let null = || {
			File::options()
				.read(true)
				.write(true)
				.open("/dev/null")
				.expect("/dev/null opens")
		};

		Term::read_write_pair(null(), null())
	}

	fn choices() -> Vec<Choice> {
		let mut choices = vec![
			Choice {
				key: 'y',
				label: Some("apply"),
				style: Style::new().green(),
				either_case: true,
			},
			Choice {
				key: 'n',
				label: Some("skip"),
				style: Style::new(),
				either_case: true,
			},
			Choice {
				key: 'p',
				label: Some("play"),
				style: Style::new().cyan(),
				either_case: true,
			},
		];
		choices.extend(('0'..='4').map(|key| Choice {
			key,
			label: None,
			style: Style::new(),
			either_case: false,
		}));
		choices.push(Choice {
			key: 'q',
			label: Some("quit"),
			style: Style::new().red(),
			either_case: true,
		});

		choices
	}

	#[test]
	fn enter_takes_the_default() {
		assert_eq!(resolve(&choices(), 'y', &Key::Enter), Some('y'));
	}

	#[test]
	fn a_key_answering_either_case_takes_both() {
		assert_eq!(resolve(&choices(), 'y', &Key::Char('Y')), Some('y'));
		assert_eq!(resolve(&choices(), 'y', &Key::Char('y')), Some('y'));
	}

	#[test]
	fn a_digit_only_matches_itself() {
		assert_eq!(resolve(&choices(), 'y', &Key::Char('3')), Some('3'));
		assert_eq!(resolve(&choices(), 'y', &Key::Char('9')), None);
	}

	#[test]
	fn the_question_lists_every_key() {
		assert_eq!(
			strip_ansi_codes(&question("apply", &choices())),
			"? apply [y/n/p/0/1/2/3/4/q] "
		);
	}

	#[test]
	fn a_prompt_with_no_terminal_to_read_refuses_instead_of_asking_forever() {
		assert!(pressed(&no_terminal(), "apply", &choices(), 'y').is_err());
	}

	#[test]
	fn a_checkbox_with_no_terminal_to_read_refuses_instead_of_failing_mid_draw() {
		let e = checked(&no_terminal(), "genre", &[String::from("pop")])
			.expect_err("no terminal is refused");

		assert!(format!("{e:?}").contains("no terminal"));
	}

	#[test]
	fn a_typed_line_is_trimmed() {
		assert_eq!(typed("  abc123 \n"), Ok("abc123"));
	}

	#[test]
	fn a_blank_line_is_refused() {
		assert!(typed("   ").is_err());
		assert!(typed("").is_err());
	}
}
