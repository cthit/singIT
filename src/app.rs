use crate::css::C;
use crate::song::Song;
use anyhow::anyhow;
use rand::seq::SliceRandom;
use rand::thread_rng;
use seed::app::cmds::timeout;
use seed::browser::util::document;
use seed::prelude::*;
use seed::{attrs, button, div, empty, error, img, input, p, span, C, IF};
use std::cmp::Reverse;

pub struct Model {
    songs: Vec<Song>,
    filtered_songs: Vec<usize>,
    show_elements: usize,
    filter_video: bool,
    filter_duets: bool,
}

const SCROLL_THRESHOLD: usize = 50;
const INITIAL_ELEM_COUNT: usize = 100;

//#[derive(Clone, Debug)]
pub enum Msg {
    Search(String),
    ToggleVideo,
    ToggleDuets,
    Shuffle,
    Scroll,
}

pub fn init(_url: Url, _orders: &mut impl Orders<Msg>) -> Model {
    let songs: Vec<Song> =
        serde_json::from_str(include_str!("../static/songs.json")).expect("parse songs");
    let mut filtered_songs: Vec<usize> = (0..songs.len()).collect();
    filtered_songs.shuffle(&mut thread_rng());

    Model {
        songs,
        filtered_songs,
        show_elements: INITIAL_ELEM_COUNT,
        filter_video: false,
        filter_duets: false,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Search(query) if query.is_empty() => update(Msg::Shuffle, model, orders),
        Msg::Search(query) => {
            model.filtered_songs.sort_by_cached_key(|&i| {
                let song = &model.songs[i];
                let top_score = Reverse(song.fuzzy_compare(&query));

                (top_score, &song.title, &song.artist, &song.song_hash)
            });
        }
        Msg::ToggleVideo => model.filter_video = !model.filter_video,
        Msg::ToggleDuets => model.filter_duets = !model.filter_duets,
        Msg::Shuffle => model.filtered_songs.shuffle(&mut thread_rng()),
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
                model.show_elements += 1;
                orders.perform_cmd(timeout(32, || Msg::Scroll));
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
                div![C![C.song_item_title], song.title.to_string()],
                div![C![C.song_item_artist], song.artist.to_string()],
            ],
            div![
                C![C.song_gizmos],
                match (&song.duet_singer_1, &song.duet_singer_2) {
                    (Some(p1), Some(p2)) => div![
                        C![C.duet_icon, C.tooltip],
                        span![
                            C![C.tooltiptext],
                            "Duet",
                            div![
                                C![C.marquee],
                                p![" 🗲 ", p1, " 🗲 ", p2, " 🗲 ", p1, " 🗲 ", p2]
                            ],
                        ],
                    ],
                    _ => empty![],
                },
                IF![song.video.is_some() => div![
                    C![C.video_icon, C.tooltip],
                    span![
                        C![C.tooltiptext],
                        "Musikvideo",
                    ],
                ]],
            ],
        ]
    };

    vec![
        div![
            C![C.song_search_bar],
            input![
                input_ev(Ev::Input, Msg::Search),
                attrs! {
                    At::Placeholder => "Search",
                },
                C![C.song_search_field],
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
                .filtered_songs
                .iter()
                .map(|&i| &model.songs[i])
                .filter(|song| !model.filter_video || song.video.is_some())
                .filter(|song| !model.filter_duets || song.duet().is_some())
                .map(song_card)
                .take(model.show_elements),
            //IF![model.show_elements < model.songs.len() => div![C![C.center, C.penguin]]],
        ],
    ]
}

const SONG_LIST_ID: &str = "song_list";

fn get_scroll() -> anyhow::Result<(i32, i32)> {
    let list = document()
        .get_element_by_id(SONG_LIST_ID)
        .ok_or(anyhow!("Failed to access song list element"))?;
    let scroll = list.scroll_top();
    let height = list.client_height();
    let max = (list.scroll_height() - height).max(0);
    Ok((scroll, max))
}
