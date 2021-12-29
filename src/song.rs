use serde::Deserialize;

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
