mod hold;

use std::sync::LazyLock;

use const_format::concatcp;

pub use hold::hold;

const APP: &str = "Alexdelia_declarative_listen";
const CONTACT: &str = "https://github.com/Alexdelia/listen";

pub const CLIENT: &str = concatcp!(APP, "-", env!("CARGO_PKG_VERSION"));

pub const USER_AGENT: &str = concatcp!(APP, "/", env!("CARGO_PKG_VERSION"), " ( ", CONTACT, " )");

static SHARED: LazyLock<ureq::Agent> = LazyLock::new(|| identified().build().into());

static STATUS_KEPT: LazyLock<ureq::Agent> =
	LazyLock::new(|| identified().http_status_as_error(false).build().into());

#[must_use]
pub fn shared() -> &'static ureq::Agent {
	&SHARED
}

#[must_use]
pub fn status_kept() -> &'static ureq::Agent {
	&STATUS_KEPT
}

fn identified() -> ureq::config::ConfigBuilder<ureq::typestate::AgentScope> {
	ureq::Agent::config_builder()
		.user_agent(USER_AGENT)
		.middleware(hold::gate)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn user_agent_is_what_music_brainz_asks_for() {
		assert_eq!(
			USER_AGENT,
			"Alexdelia_declarative_listen/0.1.0 ( https://github.com/Alexdelia/listen )"
		);
	}

	#[test]
	fn client_is_application_dash_version() {
		assert_eq!(CLIENT, "Alexdelia_declarative_listen-0.1.0");
	}

	#[test]
	fn client_holds_one_dash_so_either_split_yields_the_version() {
		assert_eq!(CLIENT.matches('-').count(), 1);
		assert_eq!(CLIENT.split_once('-'), CLIENT.rsplit_once('-'));
	}
}
