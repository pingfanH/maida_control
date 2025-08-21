use std::collections::HashMap;
use std::sync::RwLock;
use lazy_static::lazy_static;
use std::fs;
use anyhow::Result;
use crate::jsons::music_data::{MusicData, Song};

lazy_static! {
    pub static ref music_data: RwLock<MusicData> = RwLock::new(MusicData::new());
}
pub fn get_music_data(key:i32) -> Option<Song> {
    let data = music_data.read().unwrap();
    if data.is_empty() {
        load_music_data();
    }
    data.get(&key).cloned()
}
pub fn get_music_title(key: i32) -> Option<String> {
    let data = music_data.read().unwrap();
    if data.is_empty() {
        load_music_data().ok()?;
    }
    if let Some(song) = data.get(&key) {
        Some(song.title.clone())
    }else{
        None
    }
}
pub fn load_music_data() -> Result<()> {
    let data = fs::read_to_string(crate::mai_api::config::MUSIC_DB_PATH)?;
    let songs: Vec<Song> = serde_json::from_str(&data)?;
    let mut song_map:MusicData = HashMap::new();
    for song in songs {
        song_map.insert(song.id.parse::<i32>().unwrap(), song);
    }
    *music_data.write().unwrap() = song_map;
    Ok(())
}





