use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub cid: String,
    pub nick: String,
}

/// Response to `PUT /songs`
#[derive(Debug, Serialize, Deserialize)]
pub struct PutSongs {
    /// Number of songs that were not previously in the list.
    pub songs_added: usize,

    /// Number of songs that were deleted from the list.
    pub songs_deleted: usize,

    /// Number of songs that were already in the list, and *may* have had their metadata updated.
    pub songs_updated: usize,
}
