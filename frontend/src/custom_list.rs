use std::collections::{HashMap, HashSet};

use gloo_console::error;

use crate::{
    app::{Loading, Msg},
    fetch::fetch_list_of,
};

pub type CustomLists = HashMap<String, Loading<HashSet<String>>>;

pub async fn fetch_custom_song_list_index() -> Option<Msg> {
    let custom_lists: Vec<String> = match fetch_list_of("api/custom/lists").await {
        Ok(response) => response,
        Err(e) => {
            error!("Failed fetching custom song list index:", e);
            return None;
        }
    };

    Some(Msg::CustomSongLists(custom_lists))
}

pub async fn fetch_custom_song_list(list: String) -> Option<Msg> {
    let song_hashes: HashSet<String> = match fetch_list_of(format!("api/custom/list/{list}")).await {
        Ok(response) => response.into_iter().collect(),
        Err(e) => {
            error!("Failed fetching custom song list:", e);
            return None;
        }
    };

    Some(Msg::CustomSongList { list, song_hashes })
}
