mod entry;
mod parse;

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(about)]
pub struct Args {
	/// path to the ron file where the listens are declared
	#[clap(default_value = "listen.ron")]
	path: PathBuf,
}

fn main() -> hmerr::Result<()> {
	let args = Args::parse();

	let list = parse::parse(args.path)?;

	dbg!(list);

	// 	println!("Hello, world!");

	// 	let data = ron::from_str::<Vec<Entry>>(
	// 		r#"[
	//     (
	//         s: Yt("video_id_123"),
	//         q: 10,
	//     ),
	//     (
	//         s: Sc("artist_name", "song_title"),
	//         q: 5,
	//     ),
	// ]
	// "#,
	// 	);

	// 	dbg!(data);

	Ok(())
}
