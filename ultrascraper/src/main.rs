use std::{fs::create_dir, path::PathBuf, sync::Arc, vec};

use clap::{Parser, Subcommand};
use eyre::{eyre, WrapErr};
use rust_fuzzy_search::fuzzy_compare;
use serde::Serialize;
use serde_json;
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, BufReader},
    sync::mpsc,
    task,
};
#[derive(Parser)]
struct Opt {
    songs_dir: PathBuf,
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Save {
        output: PathBuf,
    },
    Post {
        server: String,
        #[clap(short, long)]
        token: String,
    },
    Admin {},
    Duplicates {
        output: PathBuf,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    let opt = Opt::parse();

    let (tx, mut rx) = mpsc::channel::<Song>(100);

    explore_dir(opt.songs_dir, Arc::new(tx));

    let mut songs = vec![];
    while let Some(song) = rx.recv().await {
        //let song = serde_json::to_string(&song).expect("song should always serialize");
        // println!("title={:?}, hash={:?}", song.title.unwrap(), song.song_hash);
        songs.push(song);
    }
    let jsongs =
        serde_json::to_string_pretty(&songs).with_context(|| "failed to serialize song list")?;
    //println!("{jsongs}");

    match opt.action {
        Action::Save { output } => {
            if !output.is_dir() {
                create_dir(&output).with_context(|| "failed to create output directory")?;
            }
            fs::write(output.join("songs.json"), jsongs)
                .await
                .with_context(|| "Failed to write to file")?;

            for song in &songs {
                if let Some(cover) = &song.cover {
                    let mut cover_from = song.path.clone();
                    cover_from.pop();
                    cover_from = cover_from.join(cover);
                    let cover_to = output.join(&song.song_hash);
                    if cover_from.is_file() {
                        std::fs::copy(&cover_from, &cover_to).with_context(|| {
                            format!("failed to copy song cover from {cover_from:?} to {cover_to:?}")
                        })?;
                    } else {
                        println!("invalid cover {cover_from:?}, song: {:?}", song.title)
                    }
                }
            }
        }
        Action::Post { server, token } => {
            let client = reqwest::Client::new();
            let res = client
                .put(format!("{server}/songs?token={token}"))
                .json(&jsongs)
                .send()
                .await?;
            if let Err(e) = res.error_for_status() {
                println!("Error puting song-json, {e:?}")
            }
            for song in &songs {
                if let Some(cover) = &song.cover {
                    let mut cover_from = song.path.clone();
                    cover_from.pop();
                    cover_from = cover_from.join(cover);
                    //let cover_to = output.join(&song.song_hash);
                    if cover_from.is_file() {
                        let img = tokio::fs::File::open(&cover_from)
                            .await
                            .wrap_err(eyre!("Failed to open {:?}", cover_from))?;
                        let img_res = client
                            .post(format!("{server}?token={token}"))
                            .body(img)
                            .send()
                            .await?;
                        if let Err(e) = img_res.error_for_status() {
                            println!("Error sending image {cover_from:?}, {e:?}")
                        }
                    } else {
                        println!("invalid cover {cover_from:?}, song: {:?}", song.title)
                    }
                }
            }
        }
        Action::Admin {} => {
            /* fs::write(output, jsongs)
            .await
            .with_context(|| "failed to write to file")?; */
            let mut no_video = vec![];
            let mut no_cover = vec![];
            let mut no_genre = vec![];
            for song in songs {
                if song.cover.is_none() {
                    let s = SmallSong {
                        path: song.path.clone(),
                        title: song.title.clone(),
                        artist: song.artist.clone(),
                    };
                    no_cover.push(s)
                }
                if song.video.is_none() && song.bg.is_none() {
                    let s = SmallSong {
                        path: song.path.clone(),
                        title: song.title.clone(),
                        artist: song.artist.clone(),
                    };
                    no_video.push(s)
                }
                if song.genre.is_none() {
                    let s = SmallSong {
                        path: song.path.clone(),
                        title: song.title.clone(),
                        artist: song.artist.clone(),
                    };
                    no_genre.push(s)
                }
            }
            let jno_video = serde_json::to_string_pretty(&no_video)
                .with_context(|| "failed to serialize song list")?;
            let jno_cover = serde_json::to_string_pretty(&no_cover)
                .with_context(|| "failed to serialize song list")?;
            let jno_genre = serde_json::to_string_pretty(&no_genre)
                .with_context(|| "failed to serialize song list")?;
            fs::write("no_video.json", jno_video)
                .await
                .with_context(|| "failed to write no_video to file")?;
            fs::write("no_cover.json", jno_cover)
                .await
                .with_context(|| "failed to write no_cover to file")?;
            fs::write("no_genre.json", jno_genre)
                .await
                .with_context(|| "failed to write no_cover to file")?;
        }
        Action::Duplicates { output } => {
            let mut dup_songs = vec![];
            for songa in &songs {
                for songb in &songs {
                    if songa.fuzzy_song_compare(songb) > 0.97 {
                        let song = SmallSong {
                            path: songa.path.clone(),
                            title: songa.title.clone(),
                            artist: songa.artist.clone(),
                        };
                        dup_songs.push(song);
                        break;
                    }
                }
            }
            let dup_jsongs = serde_json::to_string_pretty(&dup_songs)
                .with_context(|| "failed to serialize list of duplicate songs")?;
            fs::write(output, dup_jsongs)
                .await
                .with_context(|| "failed to write to file")?;
        }
    }

    Ok(())
}
impl Song {
    fn fuzzy_song_compare(&self, song: &Song) -> f32 {
        if self.path == song.path {
            return 0.0;
        }
        if let Some(a) = self.title.as_deref() {
            if let Some(b) = song.title.as_deref() {
                let score = fuzzy_compare(a, b);
                return score;
            }
        }
        0.0
    }
}
/*
fn scrape(songs_dir) -> Songs {

}

fn scrape_and_save(songs_dir, output_file_path) {
    let songs = scrape(songs_dir);

    // do file stuff to write `songs` to output_file_path
}


fn scrape_and_post(songs_dir, output_file_path) {
    let songs = scrape(songs_dir);

    // post songs to server
}*/

fn explore_dir(path: PathBuf, tx: Arc<mpsc::Sender<Song>>) {
    async fn inner(path: PathBuf, tx: Arc<mpsc::Sender<Song>>) -> eyre::Result<()> {
        let mut dir = fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let file_type = entry.file_type().await?;
            if file_type.is_dir() {
                explore_dir(entry.path(), Arc::clone(&tx));
            } else if file_type.is_file() {
                let file_name = entry.file_name();
                let file_name = file_name
                    .to_str()
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

#[derive(Debug, Default, Serialize, Clone)]
struct SmallSong {
    path: PathBuf,
    title: Option<String>,
    artist: Option<String>,
}

#[derive(Debug, Default, Serialize, Clone)]
struct Song {
    path: PathBuf,
    song_hash: String,
    title: Option<String>,
    artist: Option<String>,
    language: Option<String>,
    genre: Option<String>,
    year: Option<String>,
    mp3: Option<String>, // deprecated, change to audio in 2025
    cover: Option<String>,
    video: Option<String>,
    bpm: Option<String>,
    gap: Option<String>,
    bg: Option<String>,
}

async fn parse_file(path: PathBuf, tx: Arc<mpsc::Sender<Song>>) -> eyre::Result<()> {
    let file = BufReader::new(File::open(&path).await?);
    let mut lines = file.lines();

    let file_name = path.file_name().expect("file has a filename");
    //println!("hashing {file_name:?}");
    //println!("{:?}", file_name);
    let song_hash = md5::compute(file_name.to_string_lossy().as_bytes());
    let song_hash = format!("{song_hash:?}");

    let mut song = Song {
        path,
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
            "TITLE" => &mut song.title,
            "ARTIST" => &mut song.artist,
            "LANGUAGE" => &mut song.language,
            "GENRE" => &mut song.genre,
            "YEAR" => &mut song.year,
            "MP3" => &mut song.mp3,
            "COVER" => &mut song.cover,
            "VIDEO" => &mut song.video,
            "BPM" => &mut song.bpm,
            "GAP" => &mut song.gap,
            "BACKGROUND" => &mut song.bg,
            _ => continue,
        };

        *field = Some(value.to_string());
    }

    if song.title.is_some() {
        tx.send(song).await?;
    }

    Ok(())
}
