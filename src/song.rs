use crate::fuzzy::{self, FuzzyScore};
use crate::query::ParsedQuery;
use serde::Deserialize;
use std::cmp::max;

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub cover: Option<String>,
    pub song_hash: String,
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

    pub fn fuzzy_compare(&self, query: &ParsedQuery) -> FuzzyScore {
        let bad = || -1;

        let filter_strs = |query: Option<&str>, item: Option<&str>| {
            if let Some(query) = query {
                match item {
                    Some(item) => {
                        let score = fuzzy::compare(item.chars(), query.chars());
                        if score < fuzzy::max_score(query) / 2 {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
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
            return bad();
        }

        let mut score = FuzzyScore::default();
        if let Some(plain) = &query.plain {
            let title_score = fuzzy::compare(self.title.chars(), plain.chars());
            let artist_score = fuzzy::compare(self.artist.chars(), plain.chars());
            score = max(title_score, artist_score);
        }

        if let Some(title) = query.title {
            let new_score = fuzzy::compare(self.title.chars(), title.chars());
            score = max(score, new_score);
        }

        if let Some(artist) = query.artist {
            let new_score = fuzzy::compare(self.artist.chars(), artist.chars());
            score = max(score, new_score);
        }

        score
    }
}
