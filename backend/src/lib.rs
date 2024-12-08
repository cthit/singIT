pub mod db;
pub mod error;
pub mod route;
pub mod schema;
pub mod serialize;
pub mod util;

use std::{
    fs::{self, remove_file},
    future::{ready, Ready},
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_files::NamedFile;
use actix_web::{
    delete,
    error::{ErrorInternalServerError, ErrorUnauthorized},
    get, put,
    web::{self, Json, Query},
    FromRequest, HttpRequest,
};
use clap::Parser;
use diesel::{
    prelude::Insertable, upsert::excluded, ExpressionMethods, QueryDsl, Queryable, Selectable,
    SelectableHelper,
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use eyre::Context;
use serde::{Deserialize, Serialize};
use serialize::Ser;
use singit_lib::PutSongs;
use util::PathSafeString;

use crate::db::DbPool;

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

/// Get index.html on `/`
#[get("/")]
pub async fn root() -> actix_web::Result<NamedFile> {
    let path: &Path = "dist/index.html".as_ref();
    Ok(NamedFile::open(path)?)
}

/// Serve frontend files
pub async fn index(req: HttpRequest) -> actix_web::Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse()?;
    let path = Path::new("dist").join(path);
    Ok(NamedFile::open(path)?)
}

/// Serve song image directory
#[get("/songs")]
pub async fn songs(pool: web::Data<DbPool>) -> error::Result<Ser<Song>> {
    use schema::song::dsl::*;

    let mut db = pool.get().await?;

    let songs = song
        .select(Song::as_select())
        .load(&mut db)
        .await
        .wrap_err("Failed to query db for songs")?;

    Ok(Ser(songs))
}

/// Replace the song list, i.e. delete and add new songs.
///
/// This route requires ADMIN_TOKEN.
#[put("/songs")]
pub async fn put_songs(
    _token: Admin,
    pool: web::Data<DbPool>,
    new_songs: web::Json<Vec<Song>>,
) -> error::Result<Json<PutSongs>> {
    use schema::song::dsl::*;
    let mut db = pool.get().await.unwrap();

    let response = db
        .transaction(|mut db| {
            Box::pin(async move {
                // Get list of existing songs
                let old_songs = song
                    .select(Song::as_select())
                    .load(&mut db)
                    .await
                    .wrap_err("Failed to select all songs")?;

                // Delete songs which do not appear in songies
                let mut to_delete = vec![];
                let new_songs = new_songs.into_inner();
                let mut songs_updated = 0;
                for old in old_songs {
                    let mut delete = true;
                    for new in &new_songs {
                        if old.song_hash == *new.song_hash {
                            delete = false;
                            songs_updated += 1;
                            break;
                        }
                    }
                    if delete {
                        to_delete.push(old.song_hash)
                    }
                }

                let songs_added = new_songs.len() - songs_updated;
                let songs_deleted = diesel::delete(song.filter(song_hash.eq_any(to_delete)))
                    .execute(&mut db)
                    .await
                    .wrap_err("Failed to delete removed songs")?;

                // Upsert remaining songs into the table
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
                    .wrap_err("Failed to insert new songs")?;

                eyre::Ok(PutSongs {
                    songs_added,
                    songs_deleted,
                    songs_updated,
                })
            })
        })
        .await?;

    Ok(Json(response))
}

/// Get song image
#[get("/images/songs/{image}")]
pub async fn get_song_cover(
    path: web::Path<PathSafeString>,
    opt: web::Data<Arc<Opt>>,
) -> actix_web::Result<NamedFile> {
    let path = path.into_inner().0;
    let path = opt.covers_dir.join(path);
    Ok(NamedFile::open(path)?)
}

/// Delete all song covers.
///
/// This route requires ADMIN_TOKEN.
///
/// Returns how many files were deleted.
#[delete("/images/songs")]
pub async fn delete_song_covers(
    _token: Admin,
    opt: web::Data<Arc<Opt>>,
) -> actix_web::Result<Json<usize>> {
    // TODO: make this async
    let mut count = 0;
    for f in opt
        .covers_dir
        .read_dir()
        .map_err(ErrorInternalServerError)?
    {
        let f = f.map_err(ErrorInternalServerError)?;
        remove_file(f.path()).map_err(ErrorInternalServerError)?;
        count += 1;
    }

    // TODO: consider updating database to clear song covers

    Ok(Json(count))
}

/// Upload a song cover.
///
/// This route requires ADMIN_TOKEN.
#[put("/images/songs/{cover}")]
pub async fn put_song_cover(
    _token: Admin,
    path: web::Path<PathSafeString>,
    opt: web::Data<Arc<Opt>>,
    cover: web::Bytes,
) -> actix_web::Result<&'static str> {
    let path = path.into_inner().0;
    let path = opt.covers_dir.join(path);
    fs::write(path, cover).map_err(ErrorInternalServerError)?;
    Ok("✧*｡٩(ˊᗜˋ*)و✧*｡")
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

    /// The token required to send destructive requests, e.g. `PUT /songs`.
    ///
    /// If this is omitted, those routes will be unavailable.
    #[clap(long, env = "ADMIN_TOKEN")]
    pub admin_token: Option<String>,
}

pub struct Admin;

impl FromRequest for Admin {
    type Error = actix_web::error::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        let mut f = || {
            #[derive(Debug, Deserialize)]
            struct AdminTokenQuery {
                token: String,
            }

            let opt = web::Data::<Arc<Opt>>::from_request(req, payload)
                .into_inner()
                .map_err(ErrorInternalServerError)?
                .into_inner();

            let not_configured = Err(ErrorUnauthorized(
                "Admin token not configured. Check server config.",
            ));

            let Some(token) = &opt.admin_token else {
                return not_configured;
            };

            if token.is_empty() {
                return not_configured;
            }

            let query = Query::<AdminTokenQuery>::from_request(req, payload)
                .into_inner()
                // TODO: is this right?
                .map_err(ErrorUnauthorized)?
                .into_inner();

            if token == &query.token {
                Ok(Admin)
            } else {
                Err(ErrorUnauthorized("Invalid token"))
            }
        };

        ready(f())
    }
}
