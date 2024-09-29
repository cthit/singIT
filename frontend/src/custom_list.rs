use std::collections::{HashMap, HashSet};

use gloo_console::error;
use gloo_net::http::Request;

use crate::{
    app::{Loading, Msg},
    fetch::{fetch_list_of, FetchError},
};

pub type CustomLists = HashMap<String, Loading<HashSet<String>>>;

pub async fn fetch_custom_song_list_index() -> Option<Msg> {
    let custom_lists: Vec<String> = match fetch_list_of("custom/lists").await {
        Ok(response) => response,
        Err(e) => {
            error!("Failed fetching custom song list index:", e);
            return None;
        }
    };

    Some(Msg::CustomSongLists(custom_lists))
}

pub async fn fetch_custom_song_list(list: String) -> Option<Msg> {
    let song_hashes: HashSet<String> = match fetch_list_of(format!("custom/list/{list}")).await {
        Ok(response) => response.into_iter().collect(),
        Err(e) => {
            error!("Failed fetching custom song list:", e);
            return None;
        }
    };

    Some(Msg::CustomSongList { list, song_hashes })
}

pub async fn add_song_to_list(cid: String, song_hash: String) -> Option<Msg> {
    let result = async {
        let response = Request::put(&format!("/custom/list/{cid}/{song_hash}"))
            .send()
            .await?;

        if !response.ok() {
            return Err(FetchError::Status {
                code: response.status(),
                text: response.status_text(),
            });
        }

        Ok(())
    };

    if let Err(e) = result.await {
        error!("Error adding song to custom list:", e);
    }

    fetch_custom_song_list(cid).await
}

pub async fn remove_song_from_list(cid: String, song_hash: String) -> Option<Msg> {
    let result = async {
        let response = Request::delete(&format!("/custom/list/{cid}/{song_hash}"))
            .send()
            .await?;

        if !response.ok() {
            return Err(FetchError::Status {
                code: response.status(),
                text: response.status_text(),
            });
        }

        Ok(())
    };

    if let Err(e) = result.await {
        error!("Error removing song from custom list:", e);
    }

    fetch_custom_song_list(cid).await
}
