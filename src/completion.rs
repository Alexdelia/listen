use std::io::stdout;

use clap::CommandFactory;
use clap_complete::{Shell, generate};

use crate::args::Args;

const SHELL_WRAPPER: [&str; 3] = ["match", "outlier", "recommend"];

pub(crate) fn run(shell: Shell) {
	let mut cmd = Args::command();

	for bin in [env!("CARGO_BIN_NAME"), "run"] {
		generate(shell, &mut cmd, bin, &mut stdout());
	}

	for wrapper in SHELL_WRAPPER {
		let Some(sub) = cmd.find_subcommand(wrapper) else {
			continue;
		};

		let mut sub = sub.clone().name(wrapper).bin_name(wrapper);

		generate(shell, &mut sub, wrapper, &mut stdout());
	}
}
