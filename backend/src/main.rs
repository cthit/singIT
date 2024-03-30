mod db;
mod schema;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_files::NamedFile;
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_utils::future::{ready, Ready};
use actix_web::{
    cookie::Key,
    get,
    middleware::Logger,
    web::{self, Json, Redirect},
    App, FromRequest, HttpRequest, HttpServer, Responder,
};
use clap::Parser;
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use dotenv::dotenv;
use log::warn;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
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

const GAMMA_AUTH_STATE_KEY: &str = "GAMMA_AUTH_STATE";

#[get("/login/gamma")]
async fn login_with_gamma(opt: web::Data<Arc<Opt>>, session: Session) -> impl Responder {
    // 1. Generate state to use towards gamma
    // 2. Call gamma with values
    let state: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let client = reqwest::Client::new();
    let auth_resp = client
        .get(&opt.gamma_auth_uri)
        .query(&[
            ("response_type", "code"),
            ("client_id", &opt.gamma_client_id),
            ("redirect_uri", &opt.gamma_redirect_uri),
            ("state", &state),
            ("scope", "profile openid"),
        ])
        .build()
        .expect("Failed to build auth URI")
        .url()
        .to_string();

    session
        .insert(GAMMA_AUTH_STATE_KEY.to_string(), state)
        .expect("Failed to set state cookie");

    Redirect::to(auth_resp.to_string()).temporary()
}

#[derive(Deserialize)]
struct RedirectParams {
    state: String,
    code: String,
}

#[derive(Debug, Clone, Serialize)]
struct AuthTokenRequest {
    grant_type: String,
    redirect_uri: String,
    client_id: String,
    client_secret: String,
    code: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AuthTokenResponse {
    access_token: String,
    expires_in: i64,
    token_type: String,
}

const ACCESS_TOKEN_SESSION_KEY: &str = "access_token";

#[get("/login/gamma/redirect")]
async fn gamma_redirect(
    queries: web::Query<RedirectParams>,
    opt: web::Data<Arc<Opt>>,
    session: Session,
) -> impl Responder {
    let expected_state: String = session
        .get(GAMMA_AUTH_STATE_KEY)
        .expect("Failed to read session store")
        .expect("Failed to deserialize gamma auth state key");
    let received_state = &queries.state;
    if &expected_state != received_state {
        warn!("Expected state '{expected_state}', got state '{received_state}'");
        panic!("State does not match the expected state");
    }

    let form = AuthTokenRequest {
        grant_type: String::from("authorization_code"),
        redirect_uri: opt.gamma_redirect_uri.clone(),
        client_id: opt.gamma_client_id.clone(),
        client_secret: opt.gamma_client_secret.clone(),
        code: queries.code.clone(),
    };

    let client = reqwest::Client::new();
    let token_resp: AuthTokenResponse = client
        .post(&opt.gamma_token_uri)
        .form(&form)
        .send()
        .await
        .expect("Failed to send token request to gamma")
        .json()
        .await
        .expect("Failed to deserialize auth response content");

    session
        .insert(ACCESS_TOKEN_SESSION_KEY, token_resp.access_token)
        .expect("Failed to insert auth token in session");

    Redirect::to("/").temporary()
}

#[derive(Debug, Clone)]
struct User {
    access_token: String,
    // TODO: Implement when we support gamma me.
    // cid: String,
}

impl FromRequest for User {
    type Error = actix_web::error::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        let session = Session::from_request(req, payload)
            .into_inner()
            .expect("Failed to retrieve session");

        let access_token: String = session
            .get(ACCESS_TOKEN_SESSION_KEY)
            .expect("Failed to retrieve session access token, user not authorized")
            .expect("Failed to deserialize session key, user not authorized");

        ready(Ok(Self { access_token }))
    }
}

#[get("/auth/test")]
async fn auth_test(user: User) -> impl Responder {
    String::from("LOGGED IN!")
}

#[get("/auth/logout")]
async fn logout(session: Session) -> impl Responder {
    session.clear();

    Redirect::to("/").temporary()
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

    /// The auth URI for the auth call to gamma.
    #[clap(long, env = "GAMMA_AUTH_URI")]
    gamma_auth_uri: String,

    /// The token URI for the token call to gamma.
    #[clap(long, env = "GAMMA_TOKEN_URI")]
    gamma_token_uri: String,

    /// The me URI for the 'me' call to gamma.
    #[clap(long, env = "GAMMA_ME_URI")]
    gamma_me_uri: String,

    /// The secret key to use when encrypting cookies
    #[clap(long, env = "COOKIE_SECRET_KEY")]
    cookie_secret_key: String,
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
            let secret_key = Key::from(opt.cookie_secret_key.as_bytes());

            App::new()
                .wrap(logger)
                .wrap(SessionMiddleware::new(
                    CookieSessionStore::default(),
                    secret_key,
                ))
                .app_data(web::Data::new(db_pool.clone()))
                .app_data(web::Data::new(Arc::clone(&opt)))
                .service(root)
                .service(songs)
                .service(song_image)
                .service(custom_list)
                .service(custom_lists)
                .service(login_with_gamma)
                .service(gamma_redirect)
                .service(auth_test)
                .service(logout)
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
