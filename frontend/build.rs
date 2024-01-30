use std::fs;
use std::io;
use std::path::Path;

fn main() {
    index_default_covers().expect("index default covers");
}

/// Index all pngs in static/images/default_covers and expose the list in the build as an
/// environment variable DEFAULT_SONG_COVERS. This list includes the path to the images so the
/// frontend can fetch them.
fn index_default_covers() -> io::Result<()> {
    let mut files = vec![];
    for dir in fs::read_dir("static/images/default_covers")? {
        let path = dir?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("png") {
            let path = Path::new("/").join(path.strip_prefix("static").unwrap());
            files.push(path.to_string_lossy().to_string());
        }
    }

    let list = files.join(",");
    println!("cargo:rustc-env=DEFAULT_SONG_COVERS={list}");

    Ok(())
}
