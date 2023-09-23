use std::collections::{HashMap, HashSet};

use seed::{log, prelude::fetch};

use crate::app::{Loading, Msg};

pub type CustomLists = HashMap<String, Loading<HashSet<String>>>;

pub async fn fetch_custom_song_list_index() -> Option<Msg> {
    let response = match fetch("/custom/lists").await.and_then(|r| r.check_status()) {
        Ok(response) => response,
        Err(e) => {
            log!("error fetching custom song list index", e);
            return None;
        }
    };

    let custom_lists: Vec<String> = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            log!("error parsing custom song list index", e);
            return None;
        }
    };

    Some(Msg::CustomSongLists(custom_lists))
}

pub async fn fetch_custom_song_list(list: String) -> Option<Msg> {
    let response = match fetch(format!("/custom/list/{list}"))
        .await
        .and_then(|r| r.check_status())
    {
        Ok(response) => response,
        Err(e) => {
            log!("error fetching custom song list", e);
            return None;
        }
    };

    let song_hashes: HashSet<String> = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            log!("error parsing custom song list", e);
            return None;
        }
    };

    Some(Msg::CustomSongList { list, song_hashes })
}
