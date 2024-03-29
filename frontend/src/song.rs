use crate::app::Loading;
use crate::custom_list::CustomLists;
use crate::fuzzy::{self, FuzzyScore};
use crate::query::ParsedQuery;
use serde::Deserialize;
use std::cmp::max;

#[derive(Deserialize, Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub song_hash: String,
    pub cover: Option<String>,
    pub language: Option<String>,
    pub video: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub bpm: String,
    #[serde(rename = "duetsingerp1")]
    pub duet_singer_1: Option<String>,
    #[serde(rename = "duetsingerp2")]
    pub duet_singer_2: Option<String>,
}

impl Song {
    pub fn duet(&self) -> Option<(&str, &str)> {
        self.duet_singer_1
            .as_deref()
            .zip(self.duet_singer_2.as_deref())
    }

    pub fn fuzzy_compare(&self, query: &ParsedQuery, custom_lists: &CustomLists) -> FuzzyScore {
        let bad: FuzzyScore = -1;

        let filter_strs = |query: Option<&str>, item: Option<&str>| {
            if let Some(query) = query {
                match item {
                    Some(item) => {
                        let query_no_whitespace = query.replace(char::is_whitespace, "");
                        let item_no_whitespace = item.replace(char::is_whitespace, "");
                        let score =
                            fuzzy::compare(item_no_whitespace.chars(), query_no_whitespace.chars());
                        score == fuzzy::max_score(&query_no_whitespace)
                    }
                    None => false,
                }
            } else {
                true
            }
        };

        let filter_bool =
            |query: Option<bool>, item| !matches!(query, Some(query) if query != item);

        let filters: &[&dyn Fn() -> bool] = &[
            &|| filter_bool(query.duet, self.duet().is_some()),
            &|| filter_bool(query.video, self.video.is_some()),
            &|| filter_strs(query.language, self.language.as_deref()),
            &|| filter_strs(query.genre, self.genre.as_deref()),
            &|| filter_strs(query.year, self.year.as_deref()),
        ];

        if !filters.iter().all(|f| f()) {
            return bad;
        }

        if let Some(list) = query.list {
            let Some(Loading::Loaded(list)) = custom_lists.get(list) else {
                return bad;
            };

            if !list.contains(&self.song_hash) {
                return bad;
            }
        }

        let mut score = FuzzyScore::default();
        if let Some(plain) = &query.plain {
            let title_score = fuzzy::compare(self.title.chars(), plain.chars());
            let artist_score = fuzzy::compare(self.artist.chars(), plain.chars());
            score = max(title_score, artist_score);
        }

        if let Some(title) = &query.title {
            let new_score = fuzzy::compare(self.title.chars(), title.chars());
            score = max(score, new_score);
        }

        if let Some(artist) = &query.artist {
            let new_score = fuzzy::compare(self.artist.chars(), artist.chars());
            score = max(score, new_score);
        }

        score
    }
}
