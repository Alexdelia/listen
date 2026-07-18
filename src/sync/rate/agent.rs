use std::sync::LazyLock;

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
	ureq::Agent::new_with_config(
		ureq::Agent::config_builder()
			.http_status_as_error(false)
			.build(),
	)
});

pub(super) fn get() -> &'static ureq::Agent {
	&AGENT
}
