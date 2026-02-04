use std::collections::HashMap;
use std::env;
use std::sync::{OnceLock, RwLock};
use lazy_static::lazy_static;
use anyhow::{anyhow, Result};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySqlPool, Row};
use crate::jsons::music_data::{BasicInfo, Chart, MusicData, Song};

lazy_static! {
    pub static ref music_data: RwLock<MusicData> = RwLock::new(MusicData::new());
}
static DB_POOL: OnceLock<MySqlPool> = OnceLock::new();

async fn db_pool() -> Result<MySqlPool> {
    if let Some(pool) = DB_POOL.get() {
        return Ok(pool.clone());
    }
    dotenv::dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| anyhow!("DATABASE_URL is not set"))?;
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let _ = DB_POOL.set(pool.clone());
    Ok(pool)
}

pub async fn get_music_data(key:i32) -> Option<Song> {
    let data = music_data.read().unwrap();
    if data.is_empty() {
        drop(data);
        if load_music_data().await.is_err() {
            return None;
        }
    }
    let data = music_data.read().unwrap();
    data.get(&key).cloned()
}
pub async fn get_music_title(key: i32) -> Option<String> {
    let data = music_data.read().unwrap();
    if data.is_empty() {
        drop(data);
        if load_music_data().await.is_err() {
            return None;
        }
    }
    let data = music_data.read().unwrap();
    if let Some(song) = data.get(&key) {
        Some(song.title.clone())
    }else{
        None
    }
}
pub async fn load_music_data() -> Result<()> {
    let pool = db_pool().await?;
    let rows = sqlx::query(
        "SELECT id, title, type_field, artist, genre, bpm, `from`, flag
         FROM music_data",
    )
    .fetch_all(&pool)
    .await?;
    let mut song_map:MusicData = HashMap::new();
    for row in rows {
        let song_id: i32 = row.try_get("id")?;
        let title: String = row.try_get("title")?;
        let type_field: String = row.try_get("type_field")?;
        let artist: String = row.try_get("artist")?;
        let genre: String = row.try_get("genre")?;
        let bpm: i32 = row.try_get("bpm")?;
        let from_field: String = row.try_get("from")?;
        let flag: i32 = row.try_get("flag")?;

        let chart_rows = sqlx::query(
            "SELECT id, ds, level, level_index, charter, tap, hold, slide, break_note, touch
             FROM chart
             WHERE music_id = ?
             ORDER BY level_index ASC",
        )
        .bind(song_id)
        .fetch_all(&pool)
        .await?;

        let mut ds: Vec<f32> = Vec::new();
        let mut level: Vec<String> = Vec::new();
        let mut cids: Vec<i32> = Vec::new();
        let mut charts: Vec<Chart> = Vec::new();

        for chart_row in chart_rows {
            let chart_id: i32 = chart_row.try_get("id")?;
            let chart_ds: f64 = chart_row.try_get("ds")?;
            let chart_level: String = chart_row.try_get("level")?;
            let charter: String = chart_row.try_get("charter")?;
            let tap: i32 = chart_row.try_get("tap")?;
            let hold: i32 = chart_row.try_get("hold")?;
            let slide: i32 = chart_row.try_get("slide")?;
            let break_note: i32 = chart_row.try_get("break_note")?;
            let touch: i32 = chart_row.try_get("touch")?;

            ds.push(chart_ds as f32);
            level.push(chart_level);
            cids.push(chart_id);
            charts.push(Chart {
                notes: vec![tap, hold, slide, break_note, touch],
                charter,
            });
        }

        let basic_info = BasicInfo {
            title: title.clone(),
            artist,
            genre,
            bpm,
            release_date: "".to_string(),
            from: from_field,
            is_new: flag != 0,
        };

        let song = Song {
            id: song_id.to_string(),
            title,
            type_field,
            ds,
            level,
            cids,
            charts,
            basic_info,
        };
        song_map.insert(song_id, song);
    }
    *music_data.write().unwrap() = song_map;
    Ok(())
}




