use crate::category::Category;
use crate::css::C;
use crate::custom_list::{fetch_custom_song_list, fetch_custom_song_list_index, CustomLists};
use crate::fetch::fetch_list_of;
use crate::fuzzy::FuzzyScore;
use crate::query::ParsedQuery;
use crate::song::Song;
use gloo_console::error;
use rand::seq::SliceRandom;
use rand::thread_rng;
use seed::app::cmds::timeout;
use seed::browser::util::document;
use seed::prelude::*;
use seed::{attrs, button, div, empty, img, input, p, span, C, IF};
use std::cmp::Reverse;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};
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

    /// Which screen is currently being shown
    screen: View,

    query_placeholder: String,
    query_placeholder_len: usize,

    autotyper: Option<CmdHandle>,

    /// URLs of some defaults for songs with missing cover art.
    default_song_covers: Vec<&'static str>,
}

#[derive(Default, PartialEq)]
enum View {
    /// The main song list.
    #[default]
    Songs,

    /// The list of song categories.
    Categories,
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

    /// The user pressed the Categories button
    ToggleCategories,

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

    // get list of default song covers. see build.rs
    const DEFAULT_SONG_COVERS: &str = env!("DEFAULT_SONG_COVERS");
    let default_song_covers = DEFAULT_SONG_COVERS.split(',').collect();

    Model {
        screen: Default::default(),
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
        default_song_covers,
    }
}

fn update_song_list(model: &mut Model, orders: &mut impl Orders<Msg>) {
    model.hidden_songs = 0;
    model.shown_songs = INITIAL_ELEM_COUNT;
    scroll_to_top();

    let query_str = &model.query;
    let query = ParsedQuery::parse(query_str);
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

    if query.has_fuzzy_parameters() {
        model.songs.sort_unstable();
    } else {
        // if the user didn't input any fuzzy parameters, shuffle the results. this is stylistic
        // choice. i don't want the same results to show up at the top over and over when the user
        // didn't search for anything specific.
        let not_hidden_songs = model.songs.len() - model.hidden_songs;
        model.songs.sort_unstable_by_key(|(score, _)| *score);
        model.songs[..not_hidden_songs].shuffle(&mut thread_rng());
        autotype_song(model, orders);
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
        Msg::ToggleCategories => {
            if model.screen == View::Categories {
                model.screen = View::Songs;
            } else {
                model.screen = View::Categories;
            }
        }
        Msg::Shuffle => {
            // clear fuzzy query parameters and call update_song_list, which will shuffle the list.
            let mut query = ParsedQuery::parse(&model.query);
            query.clear_fuzzy_parameters();
            model.query = query.to_string();

            update_song_list(model, orders);
        }
        Msg::Scroll => {
            let Some((scroll, max_scroll)) = get_scroll() else {
                error!("Failed to get song list element by id:", SONG_LIST_ID);
                return;
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

pub fn view_categories(model: &Model) -> Node<Msg> {
    let category_card = |category: &Category| -> Node<Msg> {
        let title = category.title.replace(' ', "");
        div![
            C![C.category_item],
            ev(Ev::Click, move |_| Msg::Search(format!("genre:{title}"))),
            ev(Ev::Click, |_| Msg::ToggleCategories),
            div![
                C![C.category_item_info],
                div![
                    C![C.gizmo, C.icon_genre, C.tooltip],
                    span![C![C.tooltiptext], &category.title],
                ],
                div![C![C.category_item_title], &category.title],
            ],
        ]
    };

    div![
        C![C.category_list],
        attrs! {At::Id => CATEGORY_LIST_ID},
        model
            .songs
            .iter()
            .filter(|(_, song)| song.genre.is_some())
            .map(|(_, song)| Category {
                title: song.genre.clone().unwrap()
            })
            .collect::<BTreeSet<Category>>()
            .iter()
            .map(category_card)
    ]
}

pub fn view_songs(model: &Model) -> Node<Msg> {
    let song_card = |song: &Song| -> Node<Msg> {
        div![
            C![C.song_item],
            img![
                C![C.song_item_cover],
                match song.cover {
                    Some(_) =>
                        attrs! {At::Src => format!("/images/songs/{}.png", song.song_hash)},
                    None => {
                        // use a DefaultHasher to turn the song_hash string into a number we can
                        // use to give the song a psuedo-random default cover.
                        let mut hasher = DefaultHasher::new();
                        song.song_hash.hash(&mut hasher);
                        let hash = hasher.finish() as usize;
                        let cover_i = hash % model.default_song_covers.len();
                        let cover = model.default_song_covers[cover_i];
                        attrs! { At::Src => cover }
                    }
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
                        C![C.gizmo, C.icon_genre, C.tooltip],
                        span![C![C.tooltiptext], genre],
                    ],
                    None => empty![],
                },
                match &song.language {
                    Some(language) => div![
                        C![C.gizmo, C.icon_lang, C.tooltip],
                        span![C![C.tooltiptext], language],
                    ],
                    None => empty![],
                },
                IF![song.video.is_some() => div![
                    C![C.gizmo, C.icon_video, C.tooltip],
                    span![
                        C![C.tooltiptext],
                        "Musikvideo",
                    ],
                ]],
                match (&song.duet_singer_1, &song.duet_singer_2) {
                    (Some(p1), Some(p2)) => div![
                        C![C.gizmo, C.icon_duet, C.tooltip],
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
    ]
}

pub fn view(model: &Model) -> Vec<Node<Msg>> {
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
                C![C.song_sort_button, C.tooltip, C.icon_genre],
                IF![model.screen == View::Categories => C![C.song_sort_button_selected]],
                ev(Ev::Click, |_| Msg::ToggleCategories),
                span![C![C.tooltiptext], "Visa Genrer"],
            ],
            button![
                C![C.song_sort_button, C.tooltip, C.icon_duet],
                IF![model.filter_duets => C![C.song_sort_button_selected]],
                ev(Ev::Click, |_| Msg::ToggleDuets),
                span![C![C.tooltiptext], "Endast Duetter"],
            ],
            button![
                C![C.song_sort_button, C.tooltip, C.icon_video],
                IF![model.filter_video => C![C.song_sort_button_selected]],
                ev(Ev::Click, |_| Msg::ToggleVideo),
                span![C![C.tooltiptext], "Endast med Video"],
            ],
            button![
                C![C.song_sort_button, C.song_sort_button_right],
                C![C.tooltip, C.icon_shuffle],
                ev(Ev::Click, |_| Msg::Shuffle),
                span![C![C.tooltiptext], "Blanda låtar"],
            ],
        ],
        match model.screen {
            View::Songs => view_songs(model),
            View::Categories => view_categories(model),
        },
    ]
}

async fn fetch_songs() -> Option<Msg> {
    let mut songs: Vec<Song> = match fetch_list_of("/songs").await {
        Ok(response) => response,
        Err(e) => {
            error!("Error fetching songs:", e);
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
const CATEGORY_LIST_ID: &str = "category_list";

fn get_song_list_element() -> Option<Element> {
    document().get_element_by_id(SONG_LIST_ID)
}

fn scroll_to_top() {
    if let Some(elem) = get_song_list_element() {
        elem.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

fn get_scroll() -> Option<(i32, i32)> {
    let list = get_song_list_element()?;
    let scroll = list.scroll_top();
    let height = list.client_height();
    let max = (list.scroll_height() - height).max(0);
    Some((scroll, max))
}
