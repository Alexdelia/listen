use std::path::PathBuf;

use console::{Style, style};
use miette::{IntoDiagnostic, WrapErr};

use crate::{format, library};

use super::analyze::{Analysis, Record, Undeclared};
use super::cache;

const CAP: usize = 50;

pub(super) fn render(analysis: &Analysis, username: &str) {
	matched(analysis);
	median(analysis);
	outlier(&analysis.outlier);
	undeclared(&analysis.undeclared, username);
}

pub(super) fn matched(analysis: &Analysis) {
	println!(
		"{} {matched}/{declared} declared recording",
		style("matched").bold().magenta(),
		matched = analysis.matched,
		declared = analysis.declared
	);
}

pub(super) fn median(analysis: &Analysis) {
	println!(
		"\n{} {}",
		style("median listen/day per q").bold().magenta(),
		style("(declared)").dim()
	);
	for (q, count) in &analysis.declared_per_q {
		let color = format::q_style(*q);
		let percent = count * 100 / analysis.declared.max(1);

		let head = format!("{}:", color.clone().bold().apply_to(format!("q{q}")));
		let tail = style(format!("{count:>4}{percent:>3}%")).dim().to_string();
		let median = analysis
			.median
			.get(q)
			.map(|median| format!(" {:.4}", color.apply_to(median)))
			.unwrap_or_default();

		println!("{head}{median} {tail}");
	}
}

pub(super) fn outlier_header(count: usize) {
	println!(
		"\n{} {}",
		style(&count).bold().magenta(),
		style("outlier").magenta()
	);
}

fn outlier(outlier: &[Record]) {
	outlier_header(outlier.len());

	if outlier.is_empty() {
		println!("none");
		return;
	}

	for record in outlier {
		line(record);
	}
}

pub(super) fn line(record: &Record) {
	let arrow = if record.observed < record.declared {
		Style::new().red()
	} else {
		Style::new().green()
	};

	let listen = if record.listen == 0 {
		Style::new().bold().red()
	} else {
		Style::new().bold()
	};

	let observed = format::q_style(record.observed);

	println!(
		"{}{}{}{:>4}{}{:<4} {:.4} {} {}",
		format::q_style(record.declared)
			.bold()
			.apply_to(record.declared),
		arrow.bold().apply_to("->"),
		observed.clone().bold().apply_to(record.observed),
		listen.apply_to(record.listen),
		style("/").dim(),
		style(record.days).cyan(),
		observed.apply_to(record.rate),
		style(record.mbid).dim(),
		library::tag::label(record.mbid),
	);
}

pub(super) fn undeclared(undeclared: &[Undeclared], username: &str) {
	println!(
		"\n{} {}",
		style(&undeclared.len()).bold().magenta(),
		style("listen not in file").magenta()
	);

	if undeclared.is_empty() {
		println!("none");
	}

	for undeclared in undeclared.iter().take(CAP) {
		undeclared_line(undeclared);
	}

	let more = more(undeclared);

	if more > 0 {
		println!("{}", style(format!("+{more} more")).dim());
	}

	kept(undeclared, username, more > 0);
}

const fn more(undeclared: &[Undeclared]) -> usize {
	undeclared.len().saturating_sub(CAP)
}

fn kept(undeclared: &[Undeclared], username: &str, tell: bool) {
	match written(undeclared, username) {
		Ok(path) if tell => println!("{}", style(&path.display()).bold().cyan()),
		Ok(_) => {}
		Err(e) => eprintln!("{e:?}"),
	}
}

fn written(undeclared: &[Undeclared], username: &str) -> miette::Result<PathBuf> {
	cache::undeclared::write(username, &listed(undeclared)?)
}

fn listed(undeclared: &[Undeclared]) -> miette::Result<String> {
	let mut writer = csv::Writer::from_writer(Vec::new());

	for undeclared in undeclared {
		writer.serialize(undeclared).into_diagnostic()?;
	}

	let listed = writer
		.into_inner()
		.into_diagnostic()
		.wrap_err("failed to write the undeclared listen")?;

	String::from_utf8(listed).into_diagnostic()
}

fn undeclared_line(undeclared: &Undeclared) {
	println!(
		"{:>4} {} {label}",
		style(&undeclared.listen).bold(),
		style(&undeclared.mbid).dim(),
		label = library::tag::join(&undeclared.track, &undeclared.artist)
	);
}

#[cfg(test)]
mod tests {
	use super::*;

	use crate::declaration::Source;

	fn undeclared(count: usize) -> Vec<Undeclared> {
		(0..count)
			.map(|i| Undeclared {
				mbid: Source::from_u128(i as u128),
				listen: 9,
				track: "Fairy Dance".to_string(),
				artist: "UNDEAD CORPORATION".to_string(),
			})
			.collect()
	}

	#[test]
	fn the_whole_list_is_written_out_under_a_header_however_little_of_it_is_printed() {
		let listed = listed(&undeclared(CAP + 7)).unwrap_or_default();
		let mut line = listed.lines();

		assert_eq!(line.next(), Some("mbid,listen,track,artist"));
		assert_eq!(line.count(), CAP + 7);
	}

	#[test]
	fn the_cap_holds_back_what_is_printed_not_what_is_written() {
		assert_eq!(more(&undeclared(CAP)), 0);
		assert_eq!(more(&undeclared(CAP + 3)), 3);
		assert_eq!(
			listed(&undeclared(CAP)).unwrap_or_default().lines().count(),
			CAP + 1
		);
	}

	#[test]
	fn every_listen_is_one_row_of_its_own_columns() {
		let listed = listed(&undeclared(1)).unwrap_or_default();

		assert!(!listed.contains('\x1b'), "{listed}");
		assert!(
			listed
				.contains("00000000-0000-0000-0000-000000000000,9,Fairy Dance,UNDEAD CORPORATION"),
			"{listed}"
		);
	}
}
