use crate::fuzzy::{self, FuzzyScore};
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

    pub fn fuzzy_compare(&self, query: &str) -> FuzzyScore {
        let title_score = fuzzy::compare(self.title.chars(), query.chars());
        let artist_score = fuzzy::compare(self.artist.chars(), query.chars());

        max(title_score, artist_score)
    }
}
