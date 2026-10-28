pub(super) enum Outcome {
	Declined,
	Declared { url: Option<String> },
}

impl Outcome {
	pub(super) fn of(declared: bool, url: Option<&str>) -> Self {
		if declared {
			Self::Declared {
				url: url.map(str::to_string),
			}
		} else {
			Self::Declined
		}
	}
}
