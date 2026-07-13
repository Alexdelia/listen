pub(super) fn round_sec(ms: u32) -> i64 {
	i64::from(ms / 1000 + u32::from(ms % 1000 >= 500))
}

pub(super) fn fmt(sec: Option<i64>) -> String {
	match sec {
		Some(s) if s >= 0 => format!("{}:{:02}", s / 60, s % 60),
		_ => "?:??".to_string(),
	}
}
