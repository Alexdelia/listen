use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
enum Source {
    Yt(String),
    Sc(String, String),
}

#[derive(Debug, Deserialize, Serialize)]
struct Entry {
    s: Source,
    q: u8,
}

fn main() {
    println!("Hello, world!");

    let data = ron::from_str::<Vec<Entry>>(
        r#"[
    (
        s: Yt("video_id_123"),
        q: 10,
    ),
    (
        s: Sc("artist_name", "song_title"),
        q: 5,
    ),
]
"#,
    );

    dbg!(data);
}
