use std::path::Path;

use super::sparse;

const EXTENDED_TEXT: &str = "TXXX";
const TRACK_GAIN: &str = "REPLAYGAIN_TRACK_GAIN";

pub(crate) fn carried(path: &Path) -> bool {
	sparse::read(path, &[EXTENDED_TEXT]).is_ok_and(|tag| {
		tag.extended_texts()
			.any(|text| text.description.eq_ignore_ascii_case(TRACK_GAIN))
	})
}

#[cfg(test)]
mod tests {
	use id3::{Tag, TagLike, Version, frame::ExtendedText};

	use super::*;

	fn written(name: &str, extended_text: &[&str]) -> std::path::PathBuf {
		let mut tag = Tag::new();
		tag.set_title("ひみつ基地");
		for description in extended_text {
			tag.add_frame(ExtendedText {
				description: (*description).to_string(),
				value: "-4.90 dB".to_string(),
			});
		}

		let path = std::env::temp_dir().join(format!("declarative_listen_gain_{name}.mp3"));
		std::fs::write(&path, []).unwrap();
		tag.write_to_path(&path, Version::Id3v24).unwrap();
		path
	}

	#[test]
	fn a_recording_tagged_with_its_track_gain_carries_it() {
		assert!(carried(&written(
			"tagged",
			&["MusicBrainz Track Id", TRACK_GAIN]
		)));
	}

	#[test]
	fn a_lowercase_track_gain_still_counts() {
		assert!(carried(&written("lowercase", &["replaygain_track_gain"])));
	}

	#[test]
	fn a_recording_carrying_other_extended_text_only_carries_no_gain() {
		assert!(!carried(&written("other", &["MusicBrainz Track Id"])));
	}

	#[test]
	fn a_missing_file_carries_no_gain() {
		assert!(!carried(Path::new("/nonexistent/recording.mp3")));
	}
}
