use ansi::{
	DIM,
	abbrev::{B, CYA, D, G, M, R},
};

use crate::color;

use super::analyze::{Analysis, Record, Undeclared};
use super::meta;

pub(super) fn render(analysis: &Analysis) {
	matched(analysis);
	median(analysis);
	outlier(&analysis.outlier);
	undeclared(&analysis.undeclared);
}

pub(super) fn matched(analysis: &Analysis) {
	println!(
		"{B}{M}matched{D} {matched}/{declared} declared recording",
		matched = analysis.matched,
		declared = analysis.declared,
	);
}

pub(super) fn median(analysis: &Analysis) {
	println!("\n{B}{M}median listen/day per q{D} {DIM}(declared){D}");
	for (q, count) in &analysis.declared_per_q {
		let color = color::q(*q);
		let percent = count * 100 / analysis.declared.max(1);

		let head = format!("{B}{color}q{q}{D}:");
		let tail = format!("{DIM}{count:>4}{percent:>3}%{D}");
		let median = analysis
			.median
			.get(q)
			.map(|median| format!(" {color}{median:.4}{D}"))
			.unwrap_or_default();

		println!("{head}{median} {tail}");
	}
}

pub(super) fn outlier_header(count: usize) {
	println!("\n{B}{M}{count}{D} {M}outlier{D}");
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
		R
	} else {
		G
	};

	let listen_0 = if record.listen == 0 { R } else { "" };

	println!(
		"{B}{declared_color}{declared}{D}{B}{arrow}->{D}{B}{observed_color}{observed}{D}\
{B}{listen_0}{listen:>4}{D}{DIM}/{D}{CYA}{days:<4}{D} {observed_color}{rate:.4}{D} \
{DIM}{mbid}{D} {label}",
		declared_color = color::q(record.declared),
		declared = record.declared,
		observed_color = color::q(record.observed),
		observed = record.observed,
		listen = record.listen,
		days = record.days,
		rate = record.rate,
		mbid = record.mbid,
		label = meta::label(&record.mbid),
	);
}

pub(super) fn undeclared(undeclared: &[Undeclared]) {
	println!(
		"\n{B}{M}{count}{D} {M}listen not in file{D}",
		count = undeclared.len()
	);

	if undeclared.is_empty() {
		println!("none");
		return;
	}

	for undeclared in undeclared {
		undeclared_line(undeclared);
	}
}

fn undeclared_line(undeclared: &Undeclared) {
	println!(
		"{B}{listen:>4}{D} {DIM}{mbid}{D} {label}",
		listen = undeclared.listen,
		mbid = undeclared.mbid,
		label = meta::join(&undeclared.track, &undeclared.artist),
	);
}
