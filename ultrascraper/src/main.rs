use std::{path::PathBuf, sync::Arc, os::unix::prelude::OsStrExt};

use clap::Parser;
use eyre::eyre;
use tokio::{task, fs::{self, File}, io::{BufReader, AsyncBufReadExt}, sync::mpsc};

#[derive(Parser)]
struct Opt {
    songs_dir: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    let opt = Opt::parse();

    let (tx, mut rx) = mpsc::channel::<Song>(100);

    explore_dir(opt.songs_dir, Arc::new(tx));

    while let Some(song) = rx.recv().await {
        println!("title={:?}, hash={:?}", song.title.unwrap(), song.song_hash);
    }

    Ok(())
}

fn explore_dir(path: PathBuf, tx: Arc<mpsc::Sender<Song>>) {
    async fn inner(path: PathBuf, tx: Arc<mpsc::Sender<Song>>) -> eyre::Result<()> {
        let mut dir = fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let file_type = entry.file_type().await?;
            if file_type.is_dir() {
                explore_dir(entry.path(), Arc::clone(&tx));
            } else if file_type.is_file() {
                let file_name = entry.file_name();
                let file_name = file_name.to_str()
                    .ok_or_else(|| eyre!("file name not valid utf-8: {:?}", file_name))?;

                if !file_name.ends_with(".txt") {
                    continue;
                }

                parse_file(entry.path(), Arc::clone(&tx)).await?;
            }
        }

        Ok(())
    }

    task::spawn(inner(path, tx));
}


#[derive(Debug, Default)]
struct Song {
    song_hash: String,
    title: Option<String>,
    artist: Option<String>,
    language: Option<String>,
    genre: Option<String>,
    year: Option<String>,
    mp3: Option<String>,
    cover: Option<String>,
    video: Option<String>,
    bpm: Option<String>,
    gap: Option<String>,
}

async fn parse_file(path: PathBuf, tx: Arc<mpsc::Sender<Song>>) -> eyre::Result<()> {
    let file = BufReader::new(File::open(&path).await?);
    let mut lines = file.lines();

    let file_name = path.file_name().expect("file has a filename");
    println!("hashing {file_name:?}");

    let song_hash = md5::compute(file_name.as_bytes());
    let song_hash = format!("{song_hash:?}");

    let mut song = Song {
        song_hash,
        ..Default::default()
    };

    loop {
        let Some(line) = lines.next_line().await? else {
            break;
        };

        let Some(line) = line.strip_prefix('#') else {
            break;
        };

        let Some((key, value)) = line.split_once(':') else {
            break;
        };

        let field = match key {
            "TITLE"    => &mut song.title,
            "ARTIST"   => &mut song.artist ,
            "LANGUAGE" => &mut song.language ,
            "GENRE"    => &mut song.genre ,
            "YEAR"     => &mut song.year ,
            "MP3"      => &mut song.mp3 ,
            "COVER"    => &mut song.cover ,
            "VIDEO"    => &mut song.video ,
            "BPM"      => &mut song.bpm ,
            "GAP"      => &mut song.gap,
            _ => continue,
        };

        *field = Some(value.to_string());
    }

    if song.title.is_some() {
        tx.send(song).await?;
    }

    Ok(())
}
