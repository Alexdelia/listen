use ansi::abbrev::{B, D, G, M, R};

use crate::color;

use super::analyze::{Analysis, Record};
use super::meta;

pub(super) fn render(analysis: &Analysis) {
	println!(
		"{B}{M}matched{D} {matched}/{declared} declared recording",
		matched = analysis.matched,
		declared = analysis.declared,
	);

	println!("\n{B}{M}median listen/day per q{D}");
	for (q, median) in &analysis.median {
		println!(
			"{B}{color}q{q}{D}: {color}{median:.4}{D}",
			color = color::q(*q),
		);
	}

	println!(
		"\n{B}{M}{count}{D} {M}outlier{D}",
		count = analysis.outlier.len()
	);
	if analysis.outlier.is_empty() {
		println!("none");
		return;
	}

	for record in &analysis.outlier {
		line(record);
	}
}

fn line(record: &Record) {
	let arrow = if record.observed < record.declared {
		R
	} else {
		G
	};

	println!(
		"{B}{declared_color}{declared}{D}{B}{arrow}->{D}{B}{observed_color}{observed}{D} listen={listen:>4} days={days:>4} {B}{observed_color}rate={rate:.4}{D} {mbid} {label}",
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
