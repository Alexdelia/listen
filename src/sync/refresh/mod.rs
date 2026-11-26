pub(super) mod gain;
pub(super) mod metadata;

fn said(err: &[miette::Report]) {
	if err.is_empty() {
		return;
	}

	eprint!("\n\nerrors:\n\n");
	for e in err {
		eprintln!("{e:?}");
	}
}
