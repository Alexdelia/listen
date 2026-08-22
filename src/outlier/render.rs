use std::path::PathBuf;

use ansi::{
	DIM,
	abbrev::{B, CYA, D, G, M, R},
};
use hmerr::ge;

use crate::format;

use super::analyze::{Analysis, Record, Undeclared};
use super::{cache, meta};

const CAP: usize = 50;

pub(super) fn render(analysis: &Analysis, username: &str) {
	matched(analysis);
	median(analysis);
	outlier(&analysis.outlier);
	undeclared(&analysis.undeclared, username);
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
		let color = format::q_color(*q);
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
		declared_color = format::q_color(record.declared),
		declared = record.declared,
		observed_color = format::q_color(record.observed),
		observed = record.observed,
		listen = record.listen,
		days = record.days,
		rate = record.rate,
		mbid = record.mbid,
		label = meta::label(record.mbid),
	);
}

pub(super) fn undeclared(undeclared: &[Undeclared], username: &str) {
	println!(
		"\n{B}{M}{count}{D} {M}listen not in file{D}",
		count = undeclared.len()
	);

	if undeclared.is_empty() {
		println!("none");
		return;
	}

	for undeclared in undeclared.iter().take(CAP) {
		undeclared_line(undeclared);
	}

	let Some(more) = undeclared.len().checked_sub(CAP).filter(|more| *more > 0) else {
		return;
	};

	println!("{DIM}+{more} more{D}");

	match written(undeclared, username) {
		Ok(path) => println!("{B}{CYA}{path}{D}", path = path.display()),
		Err(e) => eprintln!("{e}"),
	}
}

fn written(undeclared: &[Undeclared], username: &str) -> hmerr::Result<PathBuf> {
	cache::undeclared::write(username, &listed(undeclared)?)
}

fn listed(undeclared: &[Undeclared]) -> hmerr::Result<String> {
	let mut writer = csv::Writer::from_writer(Vec::new());

	for undeclared in undeclared {
		writer.serialize(undeclared)?;
	}

	let listed = writer
		.into_inner()
		.map_err(|e| ge!(format!("{R}failed to write the undeclared listen\n{e}")))?;

	Ok(String::from_utf8(listed)?)
}

fn undeclared_line(undeclared: &Undeclared) {
	println!(
		"{B}{listen:>4}{D} {DIM}{mbid}{D} {label}",
		listen = undeclared.listen,
		mbid = undeclared.mbid,
		label = meta::join(&undeclared.track, &undeclared.artist),
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
