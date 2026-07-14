use ansi::abbrev::{B, CYA, D, G, M, R, Y};

use super::analyze::{Analysis, Record};

pub(super) fn render(analysis: &Analysis) {
	println!(
		"# {B}{M}matched{D} {matched}/{declared} declared recording",
		matched = analysis.matched,
		declared = analysis.declared,
	);

	println!("\n# {B}{M}median listen/day per q{D}");
	for (q, median) in &analysis.median {
		println!("{B}{CYA}q{q}{D}: {median:.4}");
	}

	println!(
		"\n# {B}{M}outlier{D} ({count})",
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
	let (verdict, color) = if record.observed < record.declared {
		("overrated", R)
	} else {
		("underrated", G)
	};

	println!(
		"{B}{color}{verdict:>10}{D} {B}{Y}q{declared}{D}->{B}{Y}q{observed}{D} {mbid} listen={listen:>4} days={days:>4} rate={rate:.4}",
		declared = record.declared,
		observed = record.observed,
		mbid = record.mbid,
		listen = record.listen,
		days = record.days,
		rate = record.rate,
	);
}
