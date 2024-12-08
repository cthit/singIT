use std::sync::Arc;

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    middleware::Logger,
    web::{self, PayloadConfig},
    App, HttpServer,
};
use clap::Parser;
use dotenv::dotenv;
use gamma_rust_client::config::GammaConfig;

use singit_srv::{
    db, delete_song_covers, get_song_cover, index, put_song_cover, put_songs, root, route, songs,
    Opt,
};

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
                .app_data(PayloadConfig::new(100_000_000)) // 100 MB
                .app_data(web::Data::new(db_pool.clone()))
                .app_data(web::Data::new(Arc::clone(&opt)))
                .app_data(web::Data::new(Arc::clone(&gamma_config)))
                .service(root)
                .service(songs)
                .service(put_songs)
                .service(get_song_cover)
                .service(put_song_cover)
                .service(delete_song_covers)
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
