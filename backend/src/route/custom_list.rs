use actix_web::{
    delete,
    dev::Response,
    get, put,
    web::{self, Json},
    Responder,
};
use diesel::ExpressionMethods;
use diesel::{QueryDsl, SelectableHelper};
use diesel_async::{AsyncConnection, RunQueryDsl};
use reqwest::StatusCode;

use crate::{db::DbPool, route::auth::User, schema, CustomList};

/// Get a list of all custom lists
#[get("/custom/lists")]
pub async fn list_all(pool: web::Data<DbPool>) -> impl Responder {
    use schema::custom_list::dsl::*;

    let mut db = pool.get().await.unwrap();
    let lists: Vec<String> = custom_list.select(name).load(&mut db).await.unwrap();

    Json(lists)
}

/// Get a custom list
#[get("/custom/list/{list}")]
pub async fn get_list(pool: web::Data<DbPool>, path: web::Path<String>) -> impl Responder {
    use schema::custom_list::dsl::*;
    use schema::custom_list_entry::dsl::*;

    let list_name = path.into_inner();

    let mut db = pool.get().await.unwrap();

    let list: CustomList = custom_list
        .select(CustomList::as_select())
        .filter(name.eq(list_name))
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

/// Insert a custom list entry
#[put("/custom/list/{list}/{song_hash}")]
pub async fn insert_entry(
    user: User,
    pool: web::Data<DbPool>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    use schema::custom_list::dsl::{custom_list, id, name};
    use schema::custom_list_entry::dsl::{custom_list_entry, list_id, song_hash};

    let (list_name, new_song_hash) = path.into_inner();

    let cid = &user.info.cid;
    if cid != &list_name {
        log::warn!("User {cid:?} tried to edit custom list {list_name:?}",);
        return Response::new(StatusCode::UNAUTHORIZED);
    }

    let mut db = pool.get().await.unwrap();

    let result = db
        .transaction(|db| {
            Box::pin(async move {
                diesel::insert_into(custom_list)
                    .values(name.eq(&list_name))
                    .on_conflict_do_nothing()
                    .execute(db)
                    .await?;

                let id_of_list: i32 = custom_list
                    .select(id)
                    .filter(name.eq(list_name))
                    .get_result(db)
                    .await?;

                diesel::insert_into(custom_list_entry)
                    .values((list_id.eq(id_of_list), song_hash.eq(new_song_hash)))
                    .on_conflict_do_nothing()
                    .execute(db)
                    .await
            })
        })
        .await;

    if let Err(e) = result {
        log::error!("Error inserting custom list: {e:?}");
        return Response::new(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Response::new(StatusCode::CREATED)
}

/// Delete a custom list entry
#[delete("/custom/list/{list}/{song_hash}")]
pub async fn remove_entry(
    user: User,
    pool: web::Data<DbPool>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    use schema::custom_list::dsl::{custom_list, id, name};
    use schema::custom_list_entry::dsl::{custom_list_entry, list_id, song_hash};

    let (list_name, remove_song_hash) = path.into_inner();

    let cid = &user.info.cid;
    if cid != &list_name {
        log::warn!("User {cid:?} tried to edit custom list {list_name:?}",);
        return Response::new(StatusCode::UNAUTHORIZED);
    }

    let mut db = pool.get().await.unwrap();

    log::info!("remove {remove_song_hash} from {list_name}");

    let id_of_list: i32 = custom_list
        .select(id)
        .filter(name.eq(list_name))
        .get_result(&mut db)
        .await
        .unwrap(); // TODO

    let number_of_deleted_rows = diesel::delete(custom_list_entry)
        .filter(list_id.eq(id_of_list))
        .filter(song_hash.eq(remove_song_hash))
        .execute(&mut db)
        .await
        .unwrap(); // TODO

    match number_of_deleted_rows {
        0 => Response::new(StatusCode::NOT_FOUND),
        1 => Response::new(StatusCode::OK),
        2.. => {
            log::error!("oops, i deleted multiple custom list entries");
            Response::new(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
