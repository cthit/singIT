use std::sync::Arc;

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    middleware::Logger,
    put,
    web::{self, Json},
    App, HttpRequest, HttpServer, Responder, Result,
};

use clap::Parser;
use diesel::{
    prelude::Insertable, upsert::excluded, ExpressionMethods, QueryDsl, Queryable, Selectable,
    SelectableHelper,
};
use diesel_async::RunQueryDsl;

use dotenv::dotenv;
use gamma_rust_client::config::GammaConfig;

use singit_srv::{db, index, root, route, song_image, songs, Opt};

#[put("/songs")]
async fn post_songs(pool: web::Data<DbPool>, new_songs: web::Json<Vec<Song>>) -> Result<String> {
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
