use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use anyhow::{Ok, Result, anyhow};
use serde_json::{json, Value};
use std::{collections::HashMap, env, fs};
use reqwest::redirect::Policy;

use crate::services::{load_cookies, load_latest_session, parse_favorite_music_list, parse_update_music_map};

// async fn sync_favorites_from_remote_to_db(pool: &MySqlPool) -> Result<usize> {
//     let session = load_latest_session(pool).await?
//         .ok_or_else(|| anyhow!("session not found"))?;
//     let user_key = session.user_id.to_string();
//     log::info!("[favorites-sync] user_key={}", user_key);
//     let cookies = load_cookies(pool, &user_key).await?;
//     log::info!("[favorites-sync] cookies keys: {:?}", cookies.keys().collect::<Vec<_>>());
//     let cookie_header = cookies
//         .iter()
//         .map(|(k, v)| format!("{}={}", k, v))
//         .collect::<Vec<_>>()
//         .join("; ");
//     let url = "https://maimai.wahlap.com/maimai-mobile/home/userOption/favorite/musicList";
//     let client = reqwest::Client::builder()
//         .redirect(Policy::none())
//         .build()?;
//     let res = client
//         .get(url)
//         .header("Cookie", cookie_header)
//         .header("User-Agent", "Mozilla/5.0")
//         .header("Referer", "https://maimai.wahlap.com/maimai-mobile/")
//         .send()
//         .await?;
//     log::info!("[favorites-sync] status={}", res.status());
//             if res.status().is_redirection() {
//         return Err(anyhow!("favorites musicList redirect: {}", res.status()));
//     }
//     let text = res.text().await?;

//     log::info!("[favorites-sync] body_len={}", text.len());
//   //  log::info!("[favorites-sync] body_head={}", &text.chars().take(1000).collect::<String>());

//     let items: Vec<Value> = parse_favorite_music_list(&text);
//     log::info!("[favorites-sync] items_len={}", items.len());
//     let mut song_ids: Vec<i32> = Vec::new();
//     for item in items {
//         if let Some(id_str) = item.get("musicId").and_then(|v| v.as_str()) {
//             if let Ok(id) = id_str.parse::<i32>() {
//                 if id > 0 {
//                     song_ids.push(id);
//                 }
//             }
//         }
//     }
//     log::info!("[favorites-sync] song_ids_len={}", song_ids.len());
//     if song_ids.is_empty() {
//         return Ok(0);
//     }
//     let mut tx = pool.begin().await?;
//     sqlx::query("DELETE FROM user_favorite_music WHERE open_user_id = ?")
//         .bind(&user_key)
//         .execute(&mut *tx)
//         .await?;
//     let mut inserted = 0usize;
//     for song_id in song_ids {
//         sqlx::query(
//             "INSERT INTO user_favorite_music (open_user_id, song_id) VALUES (?, ?)",
//         )
//         .bind(&user_key)
//         .bind(song_id)
//         .execute(&mut *tx)
//         .await?;
//         inserted += 1;
//     }
//     tx.commit().await?;
//     Ok(inserted)
// }

#[tokio::test]
async fn test()->Result<()>{
   let html = fs::read_to_string("/Users/pingfanh/project/maida_control/favorite_update_music.html").unwrap();

     let update_start = std::time::Instant::now();
     let (token, title_map, id_map) = parse_update_music_map(&html);
         let update_elapsed = update_start.elapsed();
    log::info!(
        "[favorites-sync] updateMusic fetched in {:?}, title_map={}, id_map={}",
        update_elapsed,
        title_map.len(),
        id_map.len()
    );
    let mut favorites = Vec::new();
     favorites.push((
            1,
            "planet dancer",
        ));
             favorites.push((
            1,
            "ナミダと流星",
        ));
             favorites.push((
            1,
            "白花の天使",
        ));
    if favorites.len() > 50 {
        return Err(anyhow!("favorites over limit: {}", favorites.len()));
    }
    let mut music_values = Vec::new();
    let mut missing = Vec::new();
    for (song_id, title) in favorites.iter() {
        if let Some(value) = id_map.get(song_id) {
            music_values.push(value.clone());
        } else if let Some(value) = title_map.get(title.to_owned()) {
            music_values.push(value.clone());
        } else {
            missing.push(format!("{}:{}", song_id, title));
        }
    }
    log::info!("{:?}", music_values);

    Ok(())

}