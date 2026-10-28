use serde::Deserialize;

const PREFIX: &str = "@nix ";
const STORE: &str = "/nix/store/";
const DERIVATION: &str = ".drv";
const LISTED: &str = "  ";

const BUILD: u64 = 105;
const COPY_PATH: u64 = 100;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
enum Line {
	Start {
		id: u64,
		#[serde(rename = "type")]
		kind: u64,
		#[serde(default)]
		fields: Vec<serde_json::Value>,
	},
	Stop {
		id: u64,
	},
	Msg {
		msg: String,
	},
	#[serde(other)]
	Other,
}

#[derive(Clone, Copy)]
enum Section {
	Build,
	Fetch,
}

#[derive(Default)]
pub(super) struct Log {
	pub(super) build: Vec<String>,
	pub(super) fetch: Vec<String>,
	section: Option<Section>,
	active: Vec<(u64, String)>,
	done: u64,
	pub(super) said: Vec<String>,
}

impl Log {
	pub(super) fn feed(&mut self, line: &str) {
		let Some(line) = line
			.strip_prefix(PREFIX)
			.and_then(|json| serde_json::from_str::<Line>(json).ok())
		else {
			return;
		};

		match line {
			Line::Start { id, kind, fields } if kind == BUILD || kind == COPY_PATH => {
				let path = fields
					.first()
					.and_then(serde_json::Value::as_str)
					.unwrap_or_default();
				self.active.push((id, name(path)));
			}
			Line::Stop { id } => {
				let before = self.active.len();
				self.active.retain(|(active, _)| *active != id);
				if self.active.len() < before {
					self.done += 1;
				}
			}
			Line::Msg { msg } => self.read(msg),
			Line::Start { .. } | Line::Other => {}
		}
	}

	fn read(&mut self, msg: String) {
		if let Some(section) = header(&msg) {
			self.section = Some(section);
			return;
		}

		if let (Some(section), Some(path)) = (self.section, msg.strip_prefix(LISTED)) {
			match section {
				Section::Build => self.build.push(name(path)),
				Section::Fetch => self.fetch.push(name(path)),
			}
			return;
		}

		self.section = None;
		self.said.push(msg);
	}

	pub(super) const fn expected(&self) -> u64 {
		(self.build.len() + self.fetch.len()) as u64
	}

	pub(super) const fn done(&self) -> u64 {
		self.done
	}

	pub(super) fn current(&self) -> Option<&str> {
		self.active.last().map(|(_, name)| name.as_str())
	}
}

fn header(msg: &str) -> Option<Section> {
	if msg.ends_with(" will be built:") {
		return Some(Section::Build);
	}

	let announces = msg.starts_with("these ") || msg.starts_with("this ");

	(announces && msg.contains(" will be fetched")).then_some(Section::Fetch)
}

fn name(path: &str) -> String {
	let path = path.trim();
	let base = path.strip_prefix(STORE).unwrap_or(path);
	let named = base.split_once('-').map_or(base, |(_, name)| name);

	named.strip_suffix(DERIVATION).unwrap_or(named).to_string()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn fed(fixture: &str) -> Log {
		let mut log = Log::default();
		for line in fixture.lines() {
			log.feed(line);
		}

		log
	}

	#[test]
	fn a_dry_run_lists_what_would_be_built_by_name() {
		let log = fed(include_str!("fixture/dry_run.log"));

		assert_eq!(log.build, ["sample-first", "sample-second"]);
		assert_eq!(log.fetch.len(), 0);
		assert_eq!(log.said.len(), 0);
	}

	#[test]
	fn a_build_counts_every_build_and_download_it_announced_as_it_finishes() {
		let log = fed(include_str!("fixture/build.log"));

		assert_eq!(log.build, ["sample-first", "sample-second"]);
		assert_eq!(log.fetch, ["sl-5.05"]);
		assert_eq!(log.expected(), 3);
		assert_eq!(log.done(), 3);
		assert_eq!(log.current(), None);
	}

	#[test]
	fn the_derivation_being_built_is_the_current_step() {
		let fixture = include_str!("fixture/build.log");
		let building = fixture
			.lines()
			.position(|line| line.contains("building '") && line.contains("sample-second"))
			.unwrap();

		let log = fed(&fixture
			.lines()
			.take(building + 1)
			.collect::<Vec<_>>()
			.join("\n"));

		assert_eq!(log.current(), Some("sample-second"));
		assert_eq!(log.done(), 2);
	}

	#[test]
	fn a_failed_build_keeps_what_nix_said_about_it() {
		let log = fed(include_str!("fixture/failure.log"));

		assert_eq!(log.build, ["sample-fail"]);
		assert_eq!(log.said.len(), 1);
		assert!(log.said[0].contains("builder failed with exit code 3"));
	}

	#[test]
	fn lines_that_are_not_nix_events_are_ignored() {
		let log = fed("plain text\n@nix not json\n@nix {\"action\":\"setPhase\",\"id\":1}");

		assert_eq!(log.expected(), 0);
		assert_eq!(log.said.len(), 0);
	}

	#[test]
	fn a_store_path_is_named_without_its_hash_or_derivation_suffix() {
		assert_eq!(
			name("/nix/store/aan2xf68walqg6kgpjxgmxiycahps9zz-nccl-2.27.drv"),
			"nccl-2.27"
		);
		assert_eq!(
			name("  /nix/store/0n7j2s7fawm2m7m3zmqapsjwv8xlq3yf-sl-5.05"),
			"sl-5.05"
		);
	}
}
