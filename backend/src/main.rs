mod db;
mod schema;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_files::NamedFile;
use actix_web::{
    get,
    middleware::Logger,
    web::{self, Json},
    App, HttpRequest, HttpServer, Responder,
};
use clap::Parser;
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

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
    id: i32,
    name: String,
}

#[get("/")]
async fn root() -> actix_web::Result<NamedFile> {
    let path: &Path = "dist/index.html".as_ref();
    Ok(NamedFile::open(path)?)
}

async fn index(req: HttpRequest) -> actix_web::Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    let path = Path::new("dist").join(path);
    Ok(NamedFile::open(path)?)
}

#[get("/images/songs/{image}")]
async fn song_image(
    path: web::Path<String>,
    opt: web::Data<Arc<Opt>>,
) -> actix_web::Result<NamedFile> {
    let image = path.into_inner();
    let path = opt.covers_dir.join(image);
    Ok(NamedFile::open(path)?)
}

#[get("/songs")]
async fn songs(pool: web::Data<DbPool>) -> impl Responder {
    use schema::song::dsl::*;

    let mut db = pool.get().await.unwrap();
    let songs = song.select(Song::as_select()).load(&mut db).await.unwrap();

    Json(songs)
}

#[get("/custom/lists")]
async fn custom_lists(pool: web::Data<DbPool>) -> impl Responder {
    use schema::custom_list::dsl::*;

    let mut db = pool.get().await.unwrap();
    let lists: Vec<String> = custom_list.select(name).load(&mut db).await.unwrap();

    Json(lists)
}

#[get("/custom/list/{list}")]
async fn custom_list(pool: web::Data<DbPool>, path: web::Path<String>) -> impl Responder {
    use schema::custom_list::dsl::*;
    use schema::custom_list_entry::dsl::*;

    let mut db = pool.get().await.unwrap();

    let list: CustomList = custom_list
        .select(CustomList::as_select())
        .filter(name.eq(&*path))
        .get_result(&mut db)
        .await
        .unwrap();

    let list_entries: Vec<String> = custom_list_entry
        .select(song_hash)
        .filter(list_id.eq(list.id))
        .load(&mut db)
        .await
        .unwrap();

    Json(list_entries)
}

#[derive(Parser)]
pub struct Opt {
    /// Address to bind to.
    #[clap(short, long, env = "BIND_ADDRESS", default_value = "0.0.0.0")]
    address: String,

    /// Port to bind to.
    #[clap(short, long, env = "BIND_PORT", default_value = "8080")]
    port: u16,

    /// Postgresql URL.
    #[clap(short, long, env = "DATABASE_URL")]
    database_url: String,

    /// Directory where song covers are stored.
    #[clap(short, long, env = "COVERS_DIR")]
    covers_dir: PathBuf,
}

#[actix_web::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    let opt = Arc::new(Opt::parse());
    env_logger::init();

    let db_pool = db::setup(&opt).await?;
    let app = {
        let opt = Arc::clone(&opt);
        move || {
            let logger = Logger::default();

            App::new()
                .wrap(logger)
                .app_data(web::Data::new(db_pool.clone()))
                .app_data(web::Data::new(Arc::clone(&opt)))
                .service(root)
                .service(songs)
                .service(song_image)
                .service(custom_list)
                .service(custom_lists)
                .route("/{filename:.*}", web::get().to(index))
        }
    };

    log::info!("listening on {}:{}", opt.address, opt.port);

    HttpServer::new(app)
        .bind((opt.address.as_str(), opt.port))?
        .run()
        .await?;

    Ok(())
}
