use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
}

pub struct SessionData {
    pub user_id: i64,
    pub open_user_id: String,
    pub session_id: i64,
    pub open_game_id: String,
    pub user_play_flag: bool,
    pub new_user_id_flag: bool,
    pub open_game_id_flag: bool,
}

#[derive(Debug, Deserialize)]
pub struct MusicApiItem {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub ds: Vec<f64>,
    pub level: Vec<String>,
    pub cids: Vec<i32>,
    pub charts: Vec<MusicApiChart>,
    pub basic_info: MusicApiBasicInfo,
}

#[derive(Debug, Deserialize)]
pub struct MusicApiChart {
    pub notes: Vec<i32>,
    pub charter: String,
}

#[derive(Debug, Deserialize)]
pub struct MusicApiBasicInfo {
    pub title: String,
    pub artist: String,
    pub genre: String,
    pub bpm: i32,
    #[serde(default)]
    pub release_date: String,
    #[serde(rename = "from")]
    pub from_field: String,
    pub is_new: bool,
}

#[derive(Debug, Deserialize)]
pub struct AliasApiResponse {
    #[serde(default)]
    pub content: Vec<AliasApiItem>,
}

#[derive(Debug, Deserialize)]
pub struct AliasApiItem {
    #[serde(rename = "SongID")]
    pub song_id: i32,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Alias")]
    pub alias: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FavoriteSetRequest {
    pub song_ids: Vec<i32>,
}
