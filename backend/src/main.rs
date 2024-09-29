mod db;
mod route;
mod schema;
mod serialize;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_files::NamedFile;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    get,
    middleware::Logger,
    web::{self},
    App, HttpRequest, HttpServer, Responder,
};
use clap::Parser;
use diesel::{QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use dotenv::dotenv;
use gamma_rust_client::config::GammaConfig;
use serde::{Deserialize, Serialize};
use serialize::Ser;

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

    Ser(songs)
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

    /// Client ID to auth against gamma.
    #[clap(long, env = "GAMMA_CLIENT_ID")]
    gamma_client_id: String,

    /// Client secret to auth against gamma.
    #[clap(long, env = "GAMMA_CLIENT_SECRET")]
    gamma_client_secret: String,

    /// Redirect URI to use to auth against gamma.
    #[clap(long, env = "GAMMA_REDIRECT_URI")]
    gamma_redirect_uri: String,

    /// API key for gamma.
    #[clap(long, env = "GAMMA_API_KEY")]
    gamma_api_key: String,

    /// The URI for gamma.
    #[clap(long, env = "GAMMA_URI")]
    gamma_uri: String,

    /// The secret key to use when encrypting cookies
    #[clap(long, env = "COOKIE_SECRET_KEY")]
    cookie_secret_key: String,
}

#[actix_web::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    let opt = Arc::new(Opt::parse());
    env_logger::init();

    let gamma_config = Arc::new(GammaConfig {
        gamma_client_secret: opt.gamma_client_secret.clone(),
        gamma_redirect_uri: opt.gamma_redirect_uri.clone(),
        gamma_client_id: opt.gamma_client_id.clone(),
        gamma_api_key: opt.gamma_api_key.clone(),
        gamma_url: opt.gamma_uri.clone(),
        scopes: "openid profile".into(),
    });

    let db_pool = db::setup(&opt).await?;
    let app = {
        let opt = Arc::clone(&opt);
        move || {
            let logger = Logger::default();
            let secret_key = Key::from(opt.cookie_secret_key.as_bytes());

            App::new()
                .wrap(logger)
                .wrap(SessionMiddleware::new(
                    CookieSessionStore::default(),
                    secret_key,
                ))
                .app_data(web::Data::new(db_pool.clone()))
                .app_data(web::Data::new(Arc::clone(&opt)))
                .app_data(web::Data::new(Arc::clone(&gamma_config)))
                .service(root)
                .service(songs)
                .service(song_image)
                .service(route::custom_list::list_all)
                .service(route::custom_list::get_list)
                .service(route::custom_list::insert_entry)
                .service(route::custom_list::remove_entry)
                .service(route::auth::user_info)
                .service(route::auth::login_with_gamma)
                .service(route::auth::gamma_redirect)
                .service(route::auth::logout)
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
