use std::{
	fs::File,
	io::{self, BufReader, Cursor, Read},
	path::Path,
};

use id3::Tag;

const MAGIC: &[u8; 3] = b"ID3";
const HEADER_LEN: usize = 10;
const ID_LEN: usize = 4;
const V3: u8 = 3;
const V4: u8 = 4;
const SYNCHSAFE_LIMIT: usize = 1 << 28;

pub(crate) fn read(path: &Path, id: &[&str]) -> id3::Result<Tag> {
	let mut reader = BufReader::new(File::open(path)?);

	kept(&mut reader, id)?.map_or_else(
		|| Tag::read_from_path(path),
		|tag| Tag::read_from2(Cursor::new(tag)),
	)
}

fn kept(reader: &mut BufReader<File>, id: &[&str]) -> io::Result<Option<Vec<u8>>> {
	let mut header = [0; HEADER_LEN];
	if !filled(reader, &mut header)? {
		return Ok(None);
	}

	let version = header[3];
	let plain = header.starts_with(MAGIC) && (version == V3 || version == V4) && header[5] == 0;
	if !plain {
		return Ok(None);
	}

	let end = synchsafe(&header[6..HEADER_LEN]);
	let mut frames = Vec::new();
	let mut offset = 0;

	while offset + HEADER_LEN <= end {
		let mut frame = [0; HEADER_LEN];
		if !filled(reader, &mut frame)? {
			return Ok(None);
		}
		if frame[0] == 0 {
			break;
		}

		let size = if version == V4 {
			synchsafe(&frame[ID_LEN..8])
		} else {
			frame_size_v3(&frame)
		};
		offset += HEADER_LEN + size;
		if offset > end {
			return Ok(None);
		}

		if id.iter().any(|id| id.as_bytes() == &frame[..ID_LEN]) {
			frames.extend_from_slice(&frame);
			let start = frames.len();
			frames.resize(start + size, 0);
			reader.read_exact(&mut frames[start..])?;
		} else {
			let Ok(skip) = i64::try_from(size) else {
				return Ok(None);
			};
			reader.seek_relative(skip)?;
		}
	}

	Ok(rebuilt(version, &frames))
}

fn filled(reader: &mut impl Read, buf: &mut [u8]) -> io::Result<bool> {
	match reader.read_exact(buf) {
		Ok(()) => Ok(true),
		Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
		Err(e) => Err(e),
	}
}

fn rebuilt(version: u8, frames: &[u8]) -> Option<Vec<u8>> {
	if frames.len() >= SYNCHSAFE_LIMIT {
		return None;
	}

	let mut tag = Vec::with_capacity(HEADER_LEN + frames.len());
	tag.extend_from_slice(MAGIC);
	tag.extend_from_slice(&[version, 0, 0]);
	tag.extend_from_slice(&synchsafe_bytes(frames.len()));
	tag.extend_from_slice(frames);

	Some(tag)
}

fn synchsafe(bytes: &[u8]) -> usize {
	bytes
		.iter()
		.fold(0, |size, byte| (size << 7) | usize::from(byte & 0x7f))
}

fn synchsafe_bytes(size: usize) -> [u8; 4] {
	[21, 14, 7, 0].map(|shift| u8::try_from((size >> shift) & 0x7f).unwrap_or_default())
}

fn frame_size_v3(frame: &[u8; HEADER_LEN]) -> usize {
	frame[ID_LEN..8]
		.iter()
		.fold(0, |size, byte| (size << 8) | usize::from(*byte))
}

#[cfg(test)]
mod tests {
	use id3::{TagLike, Version, frame::Picture, frame::PictureType};

	use super::*;

	fn scratch(name: &str) -> std::path::PathBuf {
		std::env::temp_dir().join(format!("declarative_listen_sparse_{name}.mp3"))
	}

	fn written(version: Version) -> std::path::PathBuf {
		let mut tag = Tag::new();
		tag.set_text("TALB", "album");
		tag.add_frame(Picture {
			mime_type: "image/jpeg".to_string(),
			picture_type: PictureType::CoverFront,
			description: String::new(),
			data: vec![0xff; 4096],
		});
		tag.set_title("ひみつ基地");
		tag.set_artist("結束バンド");
		tag.set_text("TSOP", "Kessoku Band");

		let path = scratch(&format!("{version:?}"));
		std::fs::write(&path, []).unwrap();
		tag.write_to_path(&path, version).unwrap();
		path
	}

	#[test]
	fn only_the_named_frames_are_kept() {
		for version in [Version::Id3v23, Version::Id3v24] {
			let path = written(version);
			let tag = read(&path, &["TIT2", "TPE1", "TSOP"]).unwrap();

			assert_eq!(tag.title(), Some("ひみつ基地"));
			assert_eq!(tag.artist(), Some("結束バンド"));
			assert_eq!(
				tag.get("TSOP").and_then(|frame| frame.content().text()),
				Some("Kessoku Band")
			);
			assert_eq!(tag.album(), None);
			assert_eq!(tag.pictures().count(), 0);
		}
	}

	#[test]
	fn a_file_carrying_no_tag_is_no_tag() {
		let path = scratch("untagged");
		std::fs::write(&path, [0xff; 64]).unwrap();

		assert!(matches!(
			read(&path, &["TIT2"]).unwrap_err().kind,
			id3::ErrorKind::NoTag
		));
	}

	#[test]
	fn a_missing_file_is_not_found() {
		assert!(matches!(
			read(Path::new("/nonexistent/recording.mp3"), &["TIT2"]).unwrap_err().kind,
			id3::ErrorKind::Io(e) if e.kind() == io::ErrorKind::NotFound
		));
	}

	#[test]
	fn a_synchsafe_size_survives_the_round_trip() {
		for size in [0, 1, 127, 128, 490_716, SYNCHSAFE_LIMIT - 1] {
			assert_eq!(synchsafe(&synchsafe_bytes(size)), size);
		}
	}
}
