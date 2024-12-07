pub mod db;
pub mod error;
pub mod route;
pub mod schema;
pub mod serialize;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_files::NamedFile;
use actix_web::{
    get, put,
    web::{self},
    HttpRequest,
};
use clap::Parser;
use diesel::{
    prelude::Insertable, upsert::excluded, ExpressionMethods, QueryDsl, Queryable, Selectable,
    SelectableHelper,
};
use diesel_async::RunQueryDsl;
use eyre::Context;
use serde::{Deserialize, Serialize};
use serialize::Ser;

use crate::db::DbPool;
use crate::error::Result;

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Queryable,
    Selectable,
    Insertable,
)]
#[diesel(table_name = crate::schema::song)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Song {
    pub song_hash: String,
    pub title: String,
    pub artist: String,
    pub cover: Option<String>,
    pub language: Option<String>,
    pub video: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub bpm: String,
    #[serde(rename = "duetsingerp1")]
    pub duet_singer_1: Option<String>,
    #[serde(rename = "duetsingerp2")]
    pub duet_singer_2: Option<String>,
}

#[derive(Serialize, Deserialize, Queryable, Selectable, Debug, Clone, Default)]
#[diesel(table_name = crate::schema::custom_list)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CustomList {
    pub id: i32,
    pub name: String,
}

#[get("/")]
pub async fn root() -> actix_web::Result<NamedFile> {
    let path: &Path = "dist/index.html".as_ref();
    Ok(NamedFile::open(path)?)
}

pub async fn index(req: HttpRequest) -> actix_web::Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse()?;
    let path = Path::new("dist").join(path);
    Ok(NamedFile::open(path)?)
}

#[get("/images/songs/{image}")]
pub async fn song_image(
    path: web::Path<String>,
    opt: web::Data<Arc<Opt>>,
) -> actix_web::Result<NamedFile> {
    let image = path.into_inner();
    let path = opt.covers_dir.join(image);
    Ok(NamedFile::open(path)?)
}

#[get("/songs")]
pub async fn songs(pool: web::Data<DbPool>) -> Result<Ser<Song>> {
    use schema::song::dsl::*;

    let mut db = pool.get().await?;

    let songs = song
        .select(Song::as_select())
        .load(&mut db)
        .await
        .wrap_err("Failed to query db for songs")?;

    Ok(Ser(songs))
}

#[put("/songs")]
pub async fn post_songs(
    pool: web::Data<DbPool>,
    new_songs: web::Json<Vec<Song>>,
) -> Result<String> {
    use schema::song::dsl::*;
    let mut db = pool.get().await.unwrap();

    // Get list of existing songs
    let old_songs = song.select(Song::as_select()).load(&mut db).await.unwrap();
    // Delete songs which do not appear in songies
    let mut to_delete = vec![];
    let new_songs = new_songs.into_inner();
    for old in old_songs {
        let mut delete = true;
        for new in &new_songs {
            if old.song_hash == *new.song_hash {
                delete = false;
                break;
            }
        }
        if delete {
            to_delete.push(old.song_hash)
        }
    }

    diesel::delete(song.filter(song_hash.eq_any(to_delete)))
        .execute(&mut db)
        .await
        .expect("failed to delete removed songs");
    // Upsert remaining songs into the table

    // Do everything above in transaction

    //let noeuht = diesel::delete(song).execute(&mut db).await.expect("Error deleting old songs");
    diesel::insert_into(song)
        .values(new_songs)
        .on_conflict(song_hash)
        .do_update()
        .set((
            artist.eq(excluded(artist)),
            title.eq(excluded(title)),
            language.eq(excluded(language)),
            genre.eq(excluded(genre)),
            year.eq(excluded(year)),
            cover.eq(excluded(cover)),
            song_hash.eq(excluded(song_hash)),
            video.eq(excluded(video)),
            bpm.eq(excluded(bpm)),
        ))
        .execute(&mut db)
        .await
        .expect("Failed adding new songs");
    Ok("hello".into())
}

#[derive(Parser)]
pub struct Opt {
    /// Address to bind to.
    #[clap(short, long, env = "BIND_ADDRESS", default_value = "0.0.0.0")]
    pub address: String,

    /// Port to bind to.
    #[clap(short, long, env = "BIND_PORT", default_value = "8080")]
    pub port: u16,

    /// Postgresql URL.
    #[clap(short, long, env = "DATABASE_URL")]
    pub database_url: String,

    /// Whether to run database migrations on startup.
    #[clap(short, long, env = "RUN_MIGRATIONS")]
    pub run_migrations: bool,

    /// Directory where song covers are stored.
    #[clap(short, long, env = "COVERS_DIR")]
    pub covers_dir: PathBuf,

    /// Client ID to auth against gamma.
    #[clap(long, env = "GAMMA_CLIENT_ID")]
    pub gamma_client_id: String,

    /// Client secret to auth against gamma.
    #[clap(long, env = "GAMMA_CLIENT_SECRET")]
    pub gamma_client_secret: String,

    /// Redirect URI to use to auth against gamma.
    #[clap(long, env = "GAMMA_REDIRECT_URI")]
    pub gamma_redirect_uri: String,

    /// API key for gamma.
    #[clap(long, env = "GAMMA_API_KEY")]
    pub gamma_api_key: String,

    /// The URI for gamma.
    #[clap(long, env = "GAMMA_URI")]
    pub gamma_uri: String,

    /// The secret key to use when encrypting cookies
    #[clap(long, env = "COOKIE_SECRET_KEY")]
    pub cookie_secret_key: String,
}
