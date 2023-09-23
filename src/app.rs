use crate::css::C;
use crate::custom_list::{fetch_custom_song_list, fetch_custom_song_list_index, CustomLists};
use crate::fuzzy::FuzzyScore;
use crate::query::ParsedQuery;
use crate::song::Song;
use anyhow::anyhow;
use rand::seq::SliceRandom;
use rand::thread_rng;
use seed::app::cmds::timeout;
use seed::browser::util::document;
use seed::{attrs, button, div, empty, error, img, input, p, span, C, IF};
use seed::{log, prelude::*};
use std::cmp::Reverse;
use std::collections::HashSet;
use web_sys::Element;

pub struct Model {
    songs: Vec<(Reverse<FuzzyScore>, Song)>,

    /// Custom song lists, lazily loaded.
    custom_lists: CustomLists,

    /// The search string.
    query: String,

    /// The number of songs currently in the dom. Goes up when the user scrolls down.
    shown_songs: usize,

    /// The number of songs that didn't match the search critera.
    hidden_songs: usize,

    /// Whether we're filtering by video.
    filter_video: bool,

    /// Whether we're filtering by duets.
    filter_duets: bool,

    query_placeholder: String,
    query_placeholder_len: usize,

    autotyper: Option<CmdHandle>,
}

#[derive(Default)]
pub enum Loading<T> {
    /// The resource has not started loading.
    #[default]
    NotLoaded,

    /// The resource is currently loading.
    InProgress,

    /// The resource has loaded.
    Loaded(T),
}

const SCROLL_THRESHOLD: usize = 50;
const INITIAL_ELEM_COUNT: usize = 100;

pub enum Msg {
    /// Loaded songs.
    Songs(Vec<Song>),

    /// Loaded custom song index.
    CustomSongLists(Vec<String>),

    /// Loaded custom song list.
    CustomSongList {
        list: String,
        song_hashes: HashSet<String>,
    },

    /// The user entered something into the search field
    Search(String),

    /// The user pressed the Toggle Video button
    ToggleVideo,

    /// The user pressed the Toggle Duets button
    ToggleDuets,

    /// The user pressed the Shuffle button
    Shuffle,

    /// The user scrolled the song list
    Scroll,

    /// Type stuff in the search input placeholder
    Autotyper,
}

pub fn init(_url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.perform_cmd(fetch_songs());
    orders.perform_cmd(fetch_custom_song_list_index());

    Model {
        songs: vec![],
        custom_lists: Default::default(),
        query: String::new(),
        hidden_songs: 0,
        shown_songs: INITIAL_ELEM_COUNT,
        filter_video: false,
        filter_duets: false,
        query_placeholder: String::from("Sök"),
        query_placeholder_len: 0,
        autotyper: Some(orders.perform_cmd_with_handle(timeout(500, || Msg::Autotyper))),
    }
}

fn update_song_list(model: &mut Model, orders: &mut impl Orders<Msg>) {
    model.hidden_songs = 0;
    model.shown_songs = INITIAL_ELEM_COUNT;
    scroll_to_top();

    if model.query.is_empty() {
        model.filter_duets = false;
        model.filter_video = false;
        update(Msg::Shuffle, model, orders);
    } else {
        let query = ParsedQuery::parse(&model.query);
        model.filter_duets = query.duet == Some(true);
        model.filter_video = query.video == Some(true);

        if let Some(name) = query.list {
            if let Some(l @ Loading::NotLoaded) = model.custom_lists.get_mut(name) {
                orders.perform_cmd(fetch_custom_song_list(name.to_string()));
                *l = Loading::InProgress;
            }
        }

        // calculate search scores & sort list
        for (score, song) in model.songs.iter_mut() {
            let new_score = song.fuzzy_compare(&query, &model.custom_lists);
            if new_score < Default::default() {
                model.hidden_songs += 1;
            }

            *score = Reverse(new_score);
        }
        model.songs.sort_unstable();
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Songs(songs) => {
            model.songs = songs
                .into_iter()
                .map(|song| (Default::default(), song))
                .collect();
        }
        Msg::CustomSongLists(lists) => {
            model.custom_lists = lists
                .into_iter()
                .map(|list| (list, Loading::NotLoaded))
                .collect();
        }
        Msg::CustomSongList { list, song_hashes } => {
            let query = ParsedQuery::parse(&model.query);
            let update_list = query.list == Some(&list);

            *model.custom_lists.entry(list).or_default() = Loading::Loaded(song_hashes);

            if update_list {
                update_song_list(model, orders);
            }
        }
        Msg::Search(query) => {
            model.query = query;
            update_song_list(model, orders);
        }
        Msg::ToggleVideo => {
            let mut query = ParsedQuery::parse(&model.query);
            query.video = match query.video {
                Some(true) => None,
                None | Some(false) => Some(true),
            };
            update(Msg::Search(query.to_string()), model, orders);
        }
        Msg::ToggleDuets => {
            let mut query = ParsedQuery::parse(&model.query);
            query.duet = match query.duet {
                Some(true) => None,
                None | Some(false) => Some(true),
            };
            update(Msg::Search(query.to_string()), model, orders);
        }
        Msg::Shuffle => {
            model.hidden_songs = 0;
            model.shown_songs = INITIAL_ELEM_COUNT;
            scroll_to_top();
            model.query.clear();
            model.songs.shuffle(&mut thread_rng());
            autotype_song(model, orders);
        }
        Msg::Scroll => {
            let (scroll, max_scroll) = match get_scroll() {
                Ok(v) => v,
                Err(e) => {
                    error!(e);
                    return;
                }
            };

            let scroll_left: i32 = max_scroll - scroll;

            // when there are fewer elements than this below the scroll viewport, add more
            const ELEMENT_HEIGHT: i32 = 48;

            if scroll_left < ELEMENT_HEIGHT * SCROLL_THRESHOLD as i32 {
                model.shown_songs += 1;
                orders.perform_cmd(timeout(32 /* ms */, || Msg::Scroll));
            }
        }
        Msg::Autotyper => {
            model.query_placeholder_len += 1;
            while !model
                .query_placeholder
                .is_char_boundary(model.query_placeholder_len)
            {
                model.query_placeholder_len += 1;
            }

            if model.query_placeholder_len < model.query_placeholder.len() {
                model.autotyper =
                    Some(orders.perform_cmd_with_handle(timeout(80, || Msg::Autotyper)));
            }
        }
    }
}

pub fn view(model: &Model) -> Vec<Node<Msg>> {
    let song_card = |song: &Song| -> Node<Msg> {
        div![
            C![C.song_item],
            img![
                C![C.song_item_cover],
                match song.cover {
                    Some(_) => attrs! {At::Src => format!("/images/songs/{}.png", song.song_hash)},
                    None => attrs! {At::Src => "/images/default_cover.png"},
                },
            ],
            div![
                C![C.song_item_info],
                div![C![C.song_item_title], &song.title],
                div![
                    C![C.song_item_artist],
                    span![&song.artist],
                    if let Some(year) = song.year.as_ref() {
                        span![" (", year, ")"]
                    } else {
                        empty![]
                    }
                ],
            ],
            div![
                C![C.song_gizmos],
                match &song.genre {
                    Some(genre) => div![
                        C![C.gizmo, C.note_icon, C.tooltip],
                        span![C![C.tooltiptext], genre],
                    ],
                    None => empty![],
                },
                match &song.language {
                    Some(language) => div![
                        C![C.gizmo, C.flag_icon, C.tooltip],
                        span![C![C.tooltiptext], language],
                    ],
                    None => empty![],
                },
                IF![song.video.is_some() => div![
                    C![C.gizmo, C.video_icon, C.tooltip],
                    span![
                        C![C.tooltiptext],
                        "Musikvideo",
                    ],
                ]],
                match (&song.duet_singer_1, &song.duet_singer_2) {
                    (Some(p1), Some(p2)) => div![
                        C![C.gizmo, C.duet_icon, C.tooltip],
                        span![
                            C![C.tooltiptext],
                            "Duet",
                            div![
                                C![C.marquee],
                                // add duplicates to get the repeating marquee effect
                                p![" 🗲 ", p1, " 🗲 ", p2, " 🗲 ", p1, " 🗲 ", p2]
                            ],
                        ],
                    ],
                    _ => empty![],
                },
            ],
        ]
    };

    vec![
        div![
            C![C.song_search_bar],
            input![
                C![C.song_search_field],
                input_ev(Ev::Input, Msg::Search),
                attrs! {
                    At::Placeholder => &model.query_placeholder[..model.query_placeholder_len],
                    At::Value => model.query,
                },
            ],
            button![
                C![C.song_sort_button, C.tooltip],
                IF![model.filter_duets => C![C.song_sort_button_selected]],
                ev(Ev::Click, |_| Msg::ToggleDuets),
                span![C![C.tooltiptext], "Endast Duetter"],
                "D",
            ],
            button![
                C![C.song_sort_button, C.tooltip],
                IF![model.filter_video => C![C.song_sort_button_selected]],
                ev(Ev::Click, |_| Msg::ToggleVideo),
                span![C![C.tooltiptext], "Endast med Video"],
                "V",
            ],
            button![
                C![C.song_sort_button, C.song_sort_button_right, C.tooltip],
                IF![model.filter_video => C![C.song_sort_button_selected]],
                ev(Ev::Click, |_| Msg::Shuffle),
                span![C![C.tooltiptext], "Blanda låtar"],
                "🔀",
            ],
        ],
        div![
            C![C.song_list],
            attrs! {At::Id => SONG_LIST_ID},
            ev(Ev::Scroll, |_| Msg::Scroll),
            model
                .songs
                .iter()
                .map(|(_, song)| song)
                .map(song_card)
                .take(model.songs.len() - model.hidden_songs)
                .take(model.shown_songs),
        ],
    ]
}

async fn fetch_songs() -> Option<Msg> {
    let response = match fetch("/songs").await.and_then(|r| r.check_status()) {
        Ok(response) => response,
        Err(e) => {
            log!("error fetching songs", e);
            return None;
        }
    };

    let mut songs: Vec<Song> = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            log!("error parsing songs", e);
            return None;
        }
    };

    songs.shuffle(&mut thread_rng());

    Some(Msg::Songs(songs))
}

pub fn autotype_song(model: &mut Model, orders: &mut impl Orders<Msg>) {
    let (_, song) = &model.songs[0];
    model.query_placeholder = ParsedQuery::random(song, &mut thread_rng()).to_string();
    model.query_placeholder_len = 0;
    model.autotyper = Some(orders.perform_cmd_with_handle(timeout(100, || Msg::Autotyper)));
}

const SONG_LIST_ID: &str = "song_list";

fn get_song_list_element() -> anyhow::Result<Element> {
    document()
        .get_element_by_id(SONG_LIST_ID)
        .ok_or_else(|| anyhow!("Failed to access song list element"))
}

fn scroll_to_top() {
    if let Ok(elem) = get_song_list_element() {
        elem.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

fn get_scroll() -> anyhow::Result<(i32, i32)> {
    let list = get_song_list_element()?;
    let scroll = list.scroll_top();
    let height = list.client_height();
    let max = (list.scroll_height() - height).max(0);
    Ok((scroll, max))
}
