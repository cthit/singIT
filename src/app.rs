use crate::css::C;
use crate::song::Song;
use anyhow::anyhow;
use seed::browser::util::document;
use seed::prelude::*;
use seed::{attrs, br, button, div, empty, error, img, input, log, p, span, C, IF};

pub struct Model {
    songs: Vec<Song>,
    show_elements: usize,
}

const SCROLL_THRESHOLD: usize = 50;
const INITIAL_ELEM_COUNT: usize = 100;

//#[derive(Clone, Debug)]
pub enum Msg {
    Search(String),
    Scroll,
}

pub fn init(_url: Url, _orders: &mut impl Orders<Msg>) -> Model {
    Model {
        songs: serde_json::from_str(include_str!("../static/songs.json"))
            .expect("failed to parsed songs"),
        show_elements: INITIAL_ELEM_COUNT,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Search(query) => {
            log!("search query");
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

            log!("scroll={}, height={}", scroll, max_scroll);
            if scroll_left < ELEMENT_HEIGHT * SCROLL_THRESHOLD as i32 {
                log!("showing more items");
                model.show_elements += 1;
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
            ],
        ]
    };

    vec![
        div![
            C![C.song_search_bar],
            div![
                input![
                    attrs! {
                        At::Placeholder => "Search",
                    },
                    C![C.song_search_field, C.tooltip],
                ],
                button![
                    C![C.song_sort_button, C.tooltip],
                    span![C![C.tooltiptext], "awawawaw", br![], "aawawaw?"],
                ],
                button![
                    C![C.song_sort_button, C.tooltip],
                    span![C![C.tooltiptext], "awawawaw"],
                ],
                button![
                    C![C.song_sort_button, C.song_sort_button_right, C.tooltip],
                    span![C![C.tooltiptext], "awawawaw"],
                ],
            ],
        ],
        div![
            C![C.song_list],
            attrs! {At::Id => SONG_LIST_ID},
            ev(Ev::Scroll, |_| Msg::Scroll),
            model.songs.iter().take(model.show_elements).map(song_card),
            IF![model.show_elements < model.songs.len() => div![C![C.center, C.penguin]]],
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
