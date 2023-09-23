use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use log::info;

use crate::Opt;

pub type DbPool = Pool<AsyncPgConnection>;

pub async fn setup(opt: &Opt) -> eyre::Result<DbPool> {
    let manager = AsyncDieselConnectionManager::new(&opt.database_url);

    info!("setting up database pool");
    let pool = Pool::builder(manager).build()?;

    info!("testing database connection");
    let _db_test = pool.get().await?;

    // TODO: migrations

    Ok(pool)
}
