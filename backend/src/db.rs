use diesel::{Connection, PgConnection};
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use eyre::{eyre, Context};
use log::info;

use crate::Opt;

pub type DbPool = Pool<AsyncPgConnection>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub async fn setup(opt: &Opt) -> eyre::Result<DbPool> {
    info!("connecting to database");
    let mut conn =
        PgConnection::establish(&opt.database_url).wrap_err("Failed to connect to database")?;

    if opt.run_migrations {
        info!("running database migrations");
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| eyre!("Failed to run database migrations: {e:?}"))?;
    } else {
        info!("skipping database migrations");
    }

    drop(conn);

    let manager = AsyncDieselConnectionManager::new(&opt.database_url);

    info!("setting up database pool");
    let pool: DbPool = Pool::builder(manager).build()?;

    // TODO: migrations

    Ok(pool)
}
