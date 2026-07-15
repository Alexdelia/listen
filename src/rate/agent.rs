pub(super) fn build() -> ureq::Agent {
	ureq::Agent::new_with_config(
		ureq::Agent::config_builder()
			.http_status_as_error(false)
			.build(),
	)
}
