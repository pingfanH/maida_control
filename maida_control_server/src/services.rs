use anyhow::{anyhow, Result};
use reqwest::header::HeaderMap as ReqwestHeaderMap;
use serde_json::{json, Value};
use sqlx::{MySqlPool, Row};
use std::{collections::{HashMap, HashSet}, io::Write, time::Duration};
use std::sync::OnceLock;
use reqwest::redirect::Policy;
use sqlx::types::chrono::{NaiveDateTime, Utc};
use tokio::net::lookup_host;
use tokio::time::timeout;

use crate::models::{AliasApiResponse, MusicApiItem, SessionData};

pub enum HomeRefreshResult {
    Ok(String),
    Expired,
}

const UPDATE_MUSIC_CACHE_TTL_SECS: i64 = 300;
const MOBILE_UA: &str = "Mozilla/5.0 (Linux; Android 13; PJA110 Build/TP1A.220905.001; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/142.0.7444.173 Mobile Safari/537.36 XWEB/1420193 MMWEBSDK/20251006 MMWEBID/3782 MicroMessenger/8.0.66.2980(0x28004252) WeChat/arm64 Weixin NetType/WIFI Language/zh_CN ABI/arm64";

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(8)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client")
    })
}

fn debug_enabled() -> bool {
    matches!(
        std::env::var("DEBUG_LOG").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

async fn log_dns_connect_timing(url: &str) {
    if !debug_enabled() {
        return;
    }
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return;
    };
    let host = match parsed.host_str() {
        Some(h) => h.to_string(),
        None => return,
    };
    let port = parsed.port_or_known_default().unwrap_or(443);
    let dns_start = std::time::Instant::now();
    let lookup = timeout(Duration::from_secs(2), lookup_host((host.as_str(), port))).await;
    match lookup {
        Ok(Ok(mut addrs)) => {
            let dns_elapsed = dns_start.elapsed();
            let addr = addrs.next();
            log::info!(
                "[net] dns host={} port={} elapsed={:?} addr={:?}",
                host, port, dns_elapsed, addr
            );
            if let Some(addr) = addr {
                let conn_start = std::time::Instant::now();
                let conn = timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(addr)).await;
                match conn {
                    Ok(Ok(_stream)) => {
                        log::info!(
                            "[net] connect host={} port={} elapsed={:?}",
                            host, port, conn_start.elapsed()
                        );
                    }
                    Ok(Err(e)) => {
                        log::info!("[net] connect failed host={} port={} err={}", host, port, e);
                    }
                    Err(_) => {
                        log::info!("[net] connect timeout host={} port={}", host, port);
                    }
                }
            }
        }
        Ok(Err(e)) => log::info!("[net] dns failed host={} port={} err={}", host, port, e),
        Err(_) => log::info!("[net] dns timeout host={} port={}", host, port),
    }
}

// Redirect target used by OAuth flow.
pub async fn get_open_url(url: &str) -> Result<String> {
    let res = http_client().get(url).send().await?;
    log::info!("[oauth] authorize status: {}", res.status());
    let location = res
        .headers()
        .get("location")
        .ok_or_else(|| anyhow!("missing location header"))?;
    Ok(location.to_str()?.to_string())
}

// Home page HTML is the source of truth for profile info.
pub async fn refresh_home_html(pool: &MySqlPool, user_key: &str) -> Result<HomeRefreshResult> {
    let (status, text, _) = match request_maimai_text(
        pool,
        user_key,
        reqwest::Method::GET,
        "https://maimai.wahlap.com/maimai-mobile/home/",
        "https://maimai.wahlap.com/maimai-mobile/",
        None,
    )
    .await
    {
        Ok(res) => res,
        Err(_) => return Ok(HomeRefreshResult::Expired),
    };
    if status.is_redirection() {
        return Ok(HomeRefreshResult::Expired);
    }
    let (title, nickname, rating) = extract_profile_from_html(&text);
    if title.is_none() && nickname.is_none() && rating.is_none() {
        return Ok(HomeRefreshResult::Expired);
    }
    Ok(HomeRefreshResult::Ok(text))
}

pub async fn load_latest_session(pool: &MySqlPool) -> Result<Option<SessionData>> {
    let row = sqlx::query(
        "SELECT user_id, open_user_id, session_id, open_game_id, user_play_flag, new_user_id_flag, open_game_id_flag
         FROM maimai_session
         ORDER BY update_time DESC
         LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(SessionData {
        user_id: row.try_get("user_id")?,
        open_user_id: row.try_get("open_user_id")?,
        session_id: row.try_get("session_id")?,
        open_game_id: row.try_get("open_game_id")?,
        user_play_flag: row.try_get("user_play_flag")?,
        new_user_id_flag: row.try_get("new_user_id_flag")?,
        open_game_id_flag: row.try_get("open_game_id_flag")?,
    }))
}

pub async fn load_session_by_user_id(
    pool: &MySqlPool,
    user_id: &str,
) -> Result<Option<SessionData>> {
    let row = sqlx::query(
        "SELECT user_id, open_user_id, session_id, open_game_id, user_play_flag, new_user_id_flag, open_game_id_flag
         FROM maimai_session
         WHERE user_id = ?
         ORDER BY update_time DESC
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(SessionData {
        user_id: row.try_get("user_id")?,
        open_user_id: row.try_get("open_user_id")?,
        session_id: row.try_get("session_id")?,
        open_game_id: row.try_get("open_game_id")?,
        user_play_flag: row.try_get("user_play_flag")?,
        new_user_id_flag: row.try_get("new_user_id_flag")?,
        open_game_id_flag: row.try_get("open_game_id_flag")?,
    }))
}

pub async fn load_cookies(pool: &MySqlPool, user_key: &str) -> Result<HashMap<String, String>> {
    let row = sqlx::query(
        "SELECT cookies_json FROM maimai_cookie WHERE open_user_id = ? LIMIT 1",
    )
    .bind(user_key)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Err(anyhow!("cookies not found"));
    };
    let cookies_json: String = row.try_get("cookies_json")?;
    let cookies: HashMap<String, String> = serde_json::from_str(&cookies_json)?;
    Ok(cookies)
}

pub async fn save_cookies(pool: &MySqlPool, user_key: &str, cookies: &HashMap<String, String>) -> Result<()> {
    let cookies_json = serde_json::to_string(cookies)?;
    sqlx::query(
        "INSERT INTO maimai_cookie (open_user_id, cookies_json)
         VALUES (?, ?)
         ON DUPLICATE KEY UPDATE
            cookies_json = VALUES(cookies_json)",
    )
    .bind(user_key)
    .bind(cookies_json)
    .execute(pool)
    .await?;
    Ok(())
}

fn apply_set_cookie(cookies: &mut HashMap<String, String>, headers: &ReqwestHeaderMap) {
    for val in headers.get_all("set-cookie").iter() {
        if let Ok(s) = val.to_str() {
            if let Some((k, v)) = s.split_once('=') {
                let v = v.split(';').next().unwrap_or("").to_string();
                cookies.insert(k.trim().to_string(), v);
            }
        }
    }
}

async fn request_maimai_text(
    pool: &MySqlPool,
    user_key: &str,
    method: reqwest::Method,
    url: &str,
    referer: &str,
    form: Option<&Vec<(String, String)>>,
) -> Result<(reqwest::StatusCode, String, ReqwestHeaderMap)> {
    let mut cookies = load_cookies(pool, user_key).await?;
    let cookie_header = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");
    log_dns_connect_timing(url).await;
    let mut rb = http_client()
        .request(method, url)
        .header("Cookie", cookie_header)
        .header("User-Agent", MOBILE_UA)
        .header("Referer", referer);
    if let Some(form) = form {
        rb = rb.form(form);
    }
    let res = rb.send().await?;
    let status = res.status();
    let headers = res.headers().clone();
    apply_set_cookie(&mut cookies, &headers);
    let _ = save_cookies(pool, user_key, &cookies).await;
    if let Some(location) = headers.get("location").and_then(|v| v.to_str().ok()) {
        if location.contains("/maimai-mobile/error/") {
            return Err(anyhow!("maimai redirect error: {}", location));
        }
    }
    let text = res.text().await?;
    Ok((status, text, headers))
}

pub async fn load_cached_html(pool: &MySqlPool, cache_key: &str, user_key: &str) -> Result<Option<String>> {
    let row = sqlx::query(
        "SELECT content FROM maimai_cache WHERE cache_key = ? AND open_user_id = ? LIMIT 1",
    )
    .bind(cache_key)
    .bind(user_key)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let content: String = row.try_get("content")?;
    Ok(Some(content))
}

pub async fn set_session_expired(pool: &MySqlPool, user_key: &str, expired: bool) -> Result<()> {
    let value = if expired { "1" } else { "0" };
    save_cache_html(pool, "session_expired", user_key, value).await
}

pub async fn get_session_expired(pool: &MySqlPool, user_key: &str) -> Result<bool> {
    Ok(load_cached_html(pool, "session_expired", user_key)
        .await?
        .map(|v| v == "1")
        .unwrap_or(false))
}

async fn load_cached_html_with_time(
    pool: &MySqlPool,
    cache_key: &str,
    user_key: &str,
) -> Result<Option<(String, NaiveDateTime)>> {
    let row = sqlx::query(
        "SELECT content, update_time FROM maimai_cache WHERE cache_key = ? AND open_user_id = ? LIMIT 1",
    )
    .bind(cache_key)
    .bind(user_key)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let content: String = row.try_get("content")?;
    let update_time: NaiveDateTime = row.try_get("update_time")?;
    Ok(Some((content, update_time)))
}

pub async fn save_cache_html(pool: &MySqlPool, cache_key: &str, user_key: &str, content: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO maimai_cache (cache_key, open_user_id, content)
         VALUES (?, ?, ?)
         ON DUPLICATE KEY UPDATE
            content = VALUES(content)",
    )
    .bind(cache_key)
    .bind(user_key)
    .bind(content)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn fetch_session(pool: &MySqlPool, user_id: &str) -> Result<Value> {
    let session = load_session_by_user_id(pool, user_id).await?;
    if let Some(s) = session {
        Ok(json!({
            "exists": true,
            "userId": s.user_id,
            "openUserId": s.open_user_id,
            "sessionId": s.session_id,
            "openGameId": s.open_game_id,
            "userPlayFlag": s.user_play_flag,
            "newUserIdFlag": s.new_user_id_flag,
            "openGameIdFlag": s.open_game_id_flag,
        }))
    } else {
        Ok(json!({ "exists": false }))
    }
}

pub async fn fetch_favorites(pool: &MySqlPool, user_id: &str) -> Result<Value> {
    let session = load_session_by_user_id(pool, user_id).await?
        .ok_or_else(|| anyhow!("session not found"))?;
    let user_key = session.open_user_id.to_string();
    let rows = sqlx::query(
        "SELECT m.id, m.title, m.artist, m.type_field
         FROM user_favorite_music f
         JOIN music_data m ON f.song_id = m.id
         WHERE f.open_user_id = ?
         ORDER BY m.id ASC",
    )
    .bind(&user_key)
    .fetch_all(pool)
    .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let type_field: String = row.try_get("type_field")?;
        items.push(json!({
            "title": row.try_get::<String, _>("title")?,
            "artist": row.try_get::<String, _>("artist")?,
            "musicType": map_music_type(&type_field),
            "image": "",
            "musicId": row.try_get::<i32, _>("id")?.to_string(),
        }));
    }
    Ok(json!({ "items": items }))
}

pub async fn refresh_favorites_cache(pool: &MySqlPool, user_key: &str) -> Result<Value> {
    let url = "https://maimai.wahlap.com/maimai-mobile/home/userOption/favorite/musicList";
    let (_status, text, _headers) = request_maimai_text(
        pool,
        user_key,
        reqwest::Method::GET,
        url,
        "https://maimai.wahlap.com/maimai-mobile/",
        None,
    )
    .await?;

    let items: Vec<Value> = parse_favorite_music_list(&text);
    let json_val = json!({ "items": items });
    let content = serde_json::to_string(&json_val)?;
    let _ = save_cache_html(pool, "favorites_items", user_key, &content).await;
    Ok(json_val)
}

pub fn map_music_type(type_field: &str) -> &'static str {
    let lower = type_field.to_lowercase();
    if lower.contains("dx") {
        "dx"
    } else if lower.contains("sd") || lower.contains("standard") {
        "standard"
    } else {
        "unknown"
    }
}

pub async fn sync_favorites_from_remote_to_db(pool: &MySqlPool, user_id: &str) -> Result<usize> {
    let session = load_session_by_user_id(pool, user_id).await?
        .ok_or_else(|| anyhow!("session not found"))?;
    let user_key = session.open_user_id.to_string();
    log::info!("[favorites-sync] user_key={}", user_key);
    let url = "https://maimai.wahlap.com/maimai-mobile/home/userOption/favorite/musicList";
    log::info!("start fetch");
    let (status, text, _headers) = request_maimai_text(
        pool,
        &user_key,
        reqwest::Method::GET,
        url,
        "https://maimai.wahlap.com/maimai-mobile/",
        None,
    )
    .await?;
    log::info!("[favorites-sync] status={}", status);
    if status.is_redirection() {
        return Err(anyhow!("favorites musicList redirect: {}", status));
    }
    log::info!("[favorites-sync] body_len={}", text.len());
    log::info!("[favorites-sync] body_head={}", &text.chars().take(200).collect::<String>());
        log::info!("start parse");
    let items: Vec<Value> = parse_favorite_music_list(&text);
    println!("{:?}",items);
    log::info!("[favorites-sync] items_len={}", items.len());
    // let mut song_ids: Vec<i32> = Vec::new();
    // for item in items {
    //     if let Some(id_str) = item.get("musicId").and_then(|v| v.as_str()) {
    //         if let Ok(id) = id_str.parse::<i32>() {
    //             if id > 0 {
    //                 song_ids.push(id);
    //             }
    //         }
    //     }
    // }
    // log::info!("[favorites-sync] song_ids_len={}", song_ids.len());
    // if song_ids.is_empty() {
    //     return Ok(0);
    // }
    let mut fetched_song_ids: HashSet<i32> = HashSet::new();
    for item in items {
        if let Some(title) = item.get("title").and_then(|v| v.as_str()) {
            if let Some(row) = sqlx::query("SELECT id FROM music_data WHERE title = ? LIMIT 1")
                .bind(title)
                .fetch_optional(pool)
                .await?
            {
                let song_id: i32 = row.try_get("id")?;
                fetched_song_ids.insert(song_id);
            }
        }
    }

    let mut tx = pool.begin().await?;
    let existing_rows = sqlx::query(
        "SELECT song_id FROM user_favorite_music WHERE open_user_id = ?",
    )
    .bind(&user_key)
    .fetch_all(&mut *tx)
    .await?;
    let mut existing_song_ids: HashSet<i32> = HashSet::new();
    for row in existing_rows {
        if let Ok(song_id) = row.try_get::<i32, _>("song_id") {
            existing_song_ids.insert(song_id);
        }
    }

    for song_id in existing_song_ids.difference(&fetched_song_ids) {
        sqlx::query(
            "DELETE FROM user_favorite_music WHERE open_user_id = ? AND song_id = ?",
        )
        .bind(&user_key)
        .bind(song_id)
        .execute(&mut *tx)
        .await?;
    }

    let mut inserted = 0usize;
    for song_id in fetched_song_ids.difference(&existing_song_ids) {
        sqlx::query(
            "INSERT INTO user_favorite_music (open_user_id, song_id) VALUES (?, ?)",
        )
        .bind(&user_key)
        .bind(song_id)
        .execute(&mut *tx)
        .await?;
        inserted += 1;
    }

    tx.commit().await?;
    log::info!("end inst");
    Ok(inserted)
}

pub async fn fetch_favorite_update_music_html(
    pool: &MySqlPool,
    user_id: &str,
) -> Result<Value> {
    let text = fetch_update_music_html(pool, user_id).await?;
    let mut file = std::fs::File::create("/Users/pingfanh/project/maida_control/favorite_update_music.html")?;
    file.write_all(text.as_bytes())?;
    Ok(json!({ "saved": true }))
}

pub async fn fetch_favorite_update_music_html_cached(
    pool: &MySqlPool,
    user_id: &str,
) -> Result<Value> {
    let session = load_session_by_user_id(pool, user_id).await?
        .ok_or_else(|| anyhow!("session not found"))?;
    let user_key = session.open_user_id.to_string();
    let cached = load_cached_html(pool, "favorite_update_music_html", &user_key).await?;
    match cached {
        Some(content) => Ok(json!({ "exists": true, "html": content })),
        None => Ok(json!({ "exists": false })),
    }
}

pub async fn fetch_music_list(pool: &MySqlPool) -> Result<Value> {
    let rows = sqlx::query(
        "SELECT m.id,
                m.title,
                m.artist,
                m.genre,
                m.type_field,
                GROUP_CONCAT(a.alias SEPARATOR ',') AS aliases
         FROM music_data m
         LEFT JOIN music_alias a ON a.song_id = m.id
         GROUP BY m.id, m.title, m.artist, m.genre, m.type_field
         ORDER BY m.id ASC",
    )
    .fetch_all(pool)
    .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let aliases: Option<String> = row.try_get("aliases").unwrap_or(None);
        items.push(json!({
            "id": row.try_get::<i32, _>("id")?,
            "title": row.try_get::<String, _>("title")?,
            "artist": row.try_get::<String, _>("artist")?,
            "genre": row.try_get::<String, _>("genre")?,
            "type": row.try_get::<String, _>("type_field")?,
            "aliases": aliases,
        }));
    }
    Ok(json!({ "items": items }))
}

pub async fn fetch_local_favorites(pool: &MySqlPool, user_id: &str) -> Result<Value> {
    let session = load_session_by_user_id(pool, user_id).await?
        .ok_or_else(|| anyhow!("session not found"))?;
    let user_key = session.open_user_id.to_string();
    let rows = sqlx::query(
        "SELECT song_id FROM user_favorite_music WHERE open_user_id = ?",
    )
    .bind(&user_key)
    .fetch_all(pool)
    .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(row.try_get::<i32, _>("song_id")?);
    }
    Ok(json!({ "items": items }))
}

pub async fn save_local_favorites(
    pool: &MySqlPool,
    user_id: &str,
    song_ids: Vec<i32>,
) -> Result<usize> {
    let session = load_session_by_user_id(pool, user_id).await?
        .ok_or_else(|| anyhow!("session not found"))?;
    let user_key = session.open_user_id.to_string();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM user_favorite_music WHERE open_user_id = ?")
        .bind(&user_key)
        .execute(&mut *tx)
        .await?;
    let mut inserted = 0usize;
    for song_id in song_ids {
        sqlx::query(
            "INSERT INTO user_favorite_music (open_user_id, song_id) VALUES (?, ?)",
        )
        .bind(&user_key)
        .bind(song_id)
        .execute(&mut *tx)
        .await?;
        inserted += 1;
    }
    tx.commit().await?;
    Ok(inserted)
}

pub fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

pub fn parse_update_music_map(html: &str) -> (Option<String>, HashMap<String, String>, HashMap<i32, String>) {
    let update_start = std::time::Instant::now();
    log::info!("start::parse_update_music_map");
    let mut map = HashMap::new();
    let mut id_map = HashMap::new();
    let mut token: Option<String> = None;
    if let Some(pos) = html.find("name=\"token\" value=\"") {
        let rest = &html[pos + "name=\"token\" value=\"".len()..];
        if let Some(end) = rest.find('\"') {
            token = Some(rest[..end].to_string());
        }
    }
    let mut pos = 0usize;
    while let Some(label_start) = html[pos..].find("<label class=\"favorite_checkbox_frame") {
        let start = pos + label_start;
        let end = html[start..].find("</label>").map(|e| start + e).unwrap_or(html.len());
        let block = &html[start..end];
        let value = extract_input_value(block, "music[]");
        let music_id = extract_input_value(block, "musicId")
            .or_else(|| extract_input_value(block, "music_id"))
            .and_then(|v| v.parse::<i32>().ok());
        let title = extract_after_marker(block, "favorite_music_name\">");
        if let (Some(value), Some(title)) = (value, title) {
            let title = decode_html_entities(title.trim());
            let value = value;
            map.insert(title.clone(), value.clone());
            if let Some(music_id) = music_id {
                id_map.insert(music_id, value);
            }
        }
        pos = end;
    }
      let update_elapsed = update_start.elapsed();
      log::info!("parse_update_music_map total:{:?}",update_elapsed);
    (token, map, id_map)
}

pub async fn fetch_update_music_html(pool: &MySqlPool, user_id: &str) -> Result<String> {
    let session = load_session_by_user_id(pool, user_id).await?
        .ok_or_else(|| anyhow!("session not found"))?;
    let user_key = session.open_user_id.to_string();
    if let Ok(Some((cached, update_time))) =
        load_cached_html_with_time(pool, "favorite_update_music_html", &user_key).await
    {
        let age_secs = (Utc::now().naive_utc() - update_time).num_seconds();
        if age_secs >= 0 && age_secs <= UPDATE_MUSIC_CACHE_TTL_SECS {
            log::info!(
                "[favorites-sync] use cached updateMusic html age={}s",
                age_secs
            );
            return Ok(cached);
        }
    }
    let url = "https://maimai.wahlap.com/maimai-mobile/home/userOption/favorite/updateMusic";
    log::info!("[favorites-sync] request updateMusic start");
    let request_start = std::time::Instant::now();
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=2 {
        let (status, text, _headers) = request_maimai_text(
            pool,
            &user_key,
            reqwest::Method::GET,
            url,
            "https://maimai.wahlap.com/maimai-mobile/",
            None,
        )
        .await?;
        log::info!(
            "[favorites-sync] request updateMusic status={} elapsed={:?} attempt={}",
            status,
            request_start.elapsed(),
            attempt
        );
        if status.is_redirection() {
            let _ = set_session_expired(pool, &user_key, true).await;
            last_err = Some(anyhow!("session expired: please re-auth (update cookies)"));
        } else if !status.is_success() {
            last_err = Some(anyhow!("favorite updateMusic http {} body: {}", status, text));
        } else {
            let _ = save_cache_html(pool, "favorite_update_music_html", &user_key, &text).await;
            return Ok(text);
        }
        if attempt < 2 {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("favorite updateMusic failed")))
}

pub async fn sync_favorites_with_remote(pool: &MySqlPool, user_id: &str) -> Result<Value> {
    log::info!("[favorites-sync] start");
    let total_start = std::time::Instant::now();
    let session = load_session_by_user_id(pool, user_id).await?
        .ok_or_else(|| anyhow!("session not found"))?;
    let user_key = session.open_user_id.to_string();
    log::info!("[favorites-sync] load favorites from db user_key={}", user_key);
    let db_start = std::time::Instant::now();
    let favorite_rows = sqlx::query(
        "SELECT f.song_id, m.title FROM user_favorite_music f
         JOIN music_data m ON f.song_id = m.id
         WHERE f.open_user_id = ?",
    )
    .bind(&user_key)
    .fetch_all(pool)
    .await?;
    let mut favorites = Vec::new();
    for row in favorite_rows {
        favorites.push((
            row.try_get::<i32, _>("song_id")?,
            row.try_get::<String, _>("title")?,
        ));
    }
    log::info!("[favorites-sync] favorites db query elapsed {:?}", db_start.elapsed());
    log::info!("[favorites-sync] favorites loaded count={}", favorites.len());
    if favorites.len() > 50 {
        return Err(anyhow!("favorites over limit: {}", favorites.len()));
    }

    log::info!("[favorites-sync] fetch updateMusic html");
    let update_start = std::time::Instant::now();
    let html = match fetch_update_music_html(pool, &user_key).await {
        Ok(html) => html,
        Err(e) => {
            log::info!(
                "[favorites-sync] fetch updateMusic html failed after {:?}: {}",
                update_start.elapsed(),
                e
            );
            return Err(e);
        }
    };
    let update_elapsed = update_start.elapsed();
    let (token, title_map, id_map) = parse_update_music_map(&html);
    log::info!(
        "[favorites-sync] updateMusic fetched in {:?}, title_map={}, id_map={}",
        update_elapsed,
        title_map.len(),
        id_map.len()
    );

    log::info!("[favorites-sync] map favorites to updateMusic values");
    let map_start = std::time::Instant::now();
    let mut music_values = Vec::new();
    let mut missing = Vec::new();
    for (song_id, title) in favorites.iter() {
        if let Some(value) = id_map.get(song_id) {
            music_values.push(value.clone());
        } else if let Some(value) = title_map.get(title) {
            music_values.push(value.clone());
        } else {
            missing.push(format!("{}:{}", song_id, title));
        }
    }
    log::info!("[favorites-sync] map elapsed {:?}", map_start.elapsed());
    if music_values.len() > 50 {
        return Err(anyhow!("favorites over limit after mapping: {}", music_values.len()));
    }

    log::info!("[favorites-sync] build post params");
    let params_start = std::time::Instant::now();
    let mut params: Vec<(String, String)> = Vec::new();
    params.push(("idx".to_string(), "99".to_string()));
    if let Some(token) = token {
        params.push(("token".to_string(), token));
    }
    for value in music_values.iter() {
        params.push(("music[]".to_string(), value.to_string()));
    }
    log::info!("[favorites-sync] build params elapsed {:?}", params_start.elapsed());
    log::info!("{:?}", params);
    log::info!("[favorites-sync] send updateMusic set");
    let post_start = std::time::Instant::now();
    let (status, body, headers) = request_maimai_text(
        pool,
        &user_key,
        reqwest::Method::POST,
        "https://maimai.wahlap.com/maimai-mobile/home/userOption/favorite/updateMusic/set",
        "https://maimai.wahlap.com/maimai-mobile/home/userOption/favorite/updateMusic",
        Some(&params),
    )
    .await?;
    let post_elapsed = post_start.elapsed();
    let header = headers
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or_default()))
        .collect::<Vec<_>>()
        .join("\n");
    log::info!(
        "[favorites-sync] updateMusic set http {} in {:?}, total {:?}, header: {}",
        status,
        post_elapsed,
        total_start.elapsed(),
        header
    );

    if !status.is_success() {
        return Err(anyhow!("favorite update set http {} body: {}", status, header));
    }
    Ok(json!({
        "updated": true,
        "count": music_values.len(),
        "missing": missing,
    }))
}

pub async fn sync_music_data(pool: &MySqlPool) -> Result<Value> {
    let music_resp = http_client()
        .get("https://www.diving-fish.com/api/maimaidxprober/music_data")
        .header("User-Agent", "MaiDaControl/1.0")
        .send()
        .await?;
    if !music_resp.status().is_success() {
        let status = music_resp.status();
        let body = music_resp.text().await.unwrap_or_default();
        return Err(anyhow!("music_data http {} body: {}", status, body));
    }
    let music_body = music_resp.text().await?;
    let music_list: Vec<MusicApiItem> = serde_json::from_str(&music_body)
        .map_err(|e| anyhow!("music_data decode error: {e}; body: {}", music_body))?;

    let mut music_upserts = 0usize;
    let mut chart_upserts = 0usize;
    for item in music_list.iter() {
        let song_id: i32 = item.id.parse().unwrap_or(0);
        if song_id == 0 {
            continue;
        }
        let flag = if item.basic_info.is_new { 1 } else { 0 };
        sqlx::query(
            "INSERT INTO music_data (id, title, type_field, alias, addition_alias, artist, genre, bpm, `from`, flag)
             VALUES (?, ?, ?, NULL, NULL, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE
                title = VALUES(title),
                type_field = VALUES(type_field),
                artist = VALUES(artist),
                genre = VALUES(genre),
                bpm = VALUES(bpm),
                `from` = VALUES(`from`),
                flag = VALUES(flag)",
        )
        .bind(song_id)
        .bind(&item.title)
        .bind(&item.type_field)
        .bind(&item.basic_info.artist)
        .bind(&item.basic_info.genre)
        .bind(item.basic_info.bpm)
        .bind(&item.basic_info.from_field)
        .bind(flag)
        .execute(pool)
        .await?;
        music_upserts += 1;

        let mut chart_ids: Vec<i32> = Vec::new();
        for (index, chart) in item.charts.iter().enumerate() {
            let level = item.level.get(index).cloned().unwrap_or_else(|| "-".to_string());
            let ds = item.ds.get(index).cloned().unwrap_or(0.0);
            let chart_id = item.cids.get(index).cloned().unwrap_or(0);
            if chart_id == 0 {
                continue;
            }
            chart_ids.push(chart_id);
            let tap = *chart.notes.get(0).unwrap_or(&0);
            let hold = *chart.notes.get(1).unwrap_or(&0);
            let slide = *chart.notes.get(2).unwrap_or(&0);
            let break_note = *chart.notes.get(3).unwrap_or(&0);
            let touch = *chart.notes.get(4).unwrap_or(&0);
            sqlx::query(
                "INSERT INTO chart (id, ds, music_id, level, level_index, charter, tap, hold, slide, break_note, touch)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE
                    ds = VALUES(ds),
                    music_id = VALUES(music_id),
                    level = VALUES(level),
                    level_index = VALUES(level_index),
                    charter = VALUES(charter),
                    tap = VALUES(tap),
                    hold = VALUES(hold),
                    slide = VALUES(slide),
                    break_note = VALUES(break_note),
                    touch = VALUES(touch)",
            )
            .bind(chart_id)
            .bind(ds)
            .bind(song_id)
            .bind(level)
            .bind(index as i32)
            .bind(&chart.charter)
            .bind(tap)
            .bind(hold)
            .bind(slide)
            .bind(break_note)
            .bind(touch)
            .execute(pool)
            .await?;
            chart_upserts += 1;
        }

        if !chart_ids.is_empty() {
            let mut sql = String::from("DELETE FROM chart WHERE music_id = ? AND id NOT IN (");
            for i in 0..chart_ids.len() {
                if i > 0 {
                    sql.push(',');
                }
                sql.push('?');
            }
            sql.push(')');
            let mut query = sqlx::query(&sql).bind(song_id);
            for id in chart_ids {
                query = query.bind(id);
            }
            let _ = query.execute(pool).await?;
        }
    }

    let alias_counts = sync_alias_data(pool, http_client()).await?;
    Ok(json!({
        "music_upserts": music_upserts,
        "chart_upserts": chart_upserts,
        "alias_upserts": alias_counts
    }))
}

pub async fn sync_alias_data(pool: &MySqlPool, client: &reqwest::Client) -> Result<usize> {
    let alias_url = "https://www.yuzuchan.moe/api/maimaidx/maimaidxalias";
    let alias_fallback_url = "https://api.yuzuchan.moe/maimaidx/maimaidxalias";
    let response = client
        .get(alias_url)
        .header("User-Agent", "MaiDaControl/1.0")
        .header("Referer", "https://www.yuzuchan.moe/")
        .send()
        .await;
    let alias_response: AliasApiResponse = match response {
        Ok(res) => {
            let status = res.status();
            let body = res.text().await?;
            if !status.is_success() {
                let res = client
                    .get(alias_fallback_url)
                    .header("User-Agent", "MaiDaControl/1.0")
                    .header("Referer", "https://api.yuzuchan.moe/")
                    .send()
                    .await?;
                let status = res.status();
                let body = res.text().await?;
                if !status.is_success() {
                    return Err(anyhow!("alias http {} body: {}", status, body));
                }
                serde_json::from_str(&body)
                    .map_err(|e| anyhow!("alias decode error: {e}; body: {}", body))?
            } else {
                serde_json::from_str(&body)
                    .map_err(|e| anyhow!("alias decode error: {e}; body: {}", body))?
            }
        }
        Err(_) => {
            let res = client
                .get(alias_fallback_url)
                .header("User-Agent", "MaiDaControl/1.0")
                .header("Referer", "https://api.yuzuchan.moe/")
                .send()
                .await?;
            let status = res.status();
            let body = res.text().await?;
            if !status.is_success() {
                return Err(anyhow!("alias http {} body: {}", status, body));
            }
            serde_json::from_str(&body)
                .map_err(|e| anyhow!("alias decode error: {e}; body: {}", body))?
        }
    };

    let mut inserted = 0usize;
    for item in alias_response.content.iter() {
        sqlx::query("DELETE FROM music_alias WHERE song_id = ?")
            .bind(item.song_id)
            .execute(pool)
            .await?;
        for alias in item.alias.iter() {
            if alias.trim().is_empty() {
                continue;
            }
            sqlx::query(
                "INSERT INTO music_alias (song_id, alias)
                 VALUES (?, ?)
                 ON DUPLICATE KEY UPDATE alias = VALUES(alias)",
            )
            .bind(item.song_id)
            .bind(alias)
            .execute(pool)
            .await?;
            inserted += 1;
        }
    }
    Ok(inserted)
}

pub fn parse_favorite_music_list(html: &str) -> Vec<Value> {
    let mut items = Vec::new();
    let mut pos = 0usize;

    while let Some(rel_start) = html[pos..].find("<div class=\"basic_block") {
        let start = pos + rel_start;

        let next_start = html[start + 1..]
            .find("<div class=\"basic_block")
            .map(|e| start + 1 + e)
            .unwrap_or_else(|| html.len());
        let block = &html[start..next_start];

        let title = extract_after_marker(block, "f_15\">");
        let artist = extract_after_marker(block, "f_12\">");
        let img = extract_img_src_by_class(block, "music_img");
        // let music_id = extract_input_value(block, "musicId");
        // let order_id = extract_input_value(block, "orderId");
        // let token = extract_input_value(block, "token");
        let has_dx = block.contains("music_dx");
        let has_standard = block.contains("music_standard");
        let music_type = if has_dx && has_standard {
            "both"
        } else if has_dx {
            "dx"
        } else if has_standard {
            "standard"
        } else {
            "unknown"
        };

        if let (Some(title), Some(artist)) = (title, artist) {
            items.push(json!({
                "title": title,
                "artist": artist,
                "musicType": music_type,
                "image": img.unwrap_or_default(),
                // "musicId": music_id.unwrap_or_default(),
                //"orderId": order_id.unwrap_or_default(),
                //"token": token.unwrap_or_default()
            }));
        }

        pos = next_start;
    }

    items
}

pub fn extract_between(haystack: &str, start: &str, end: &str) -> Option<String> {
    let s = haystack.find(start)?;
    let rest = &haystack[s + start.len()..];
    let e = rest.find(end)?;
    Some(rest[..e].trim().to_string())
}

pub fn extract_profile_from_html(html: &str) -> (Option<String>, Option<String>, Option<String>) {
    let title = html.find("trophy_inner_block")
        .and_then(|idx| {
            let sub = &html[idx..];
            extract_between(sub, "<span>", "</span>")
        });
    let nickname = html.find("name_block")
        .and_then(|idx| {
            let sub = &html[idx..];
            extract_between(sub, ">", "</div>")
        });
    let rating = html.find("rating_block")
        .and_then(|idx| {
            let sub = &html[idx..];
            extract_between(sub, ">", "</div>")
        });
    (title, nickname, rating)
}

pub fn extract_after_marker(block: &str, marker: &str) -> Option<String> {
    let pos = block.find(marker)?;
    let start = pos + marker.len();
    let rest = &block[start..];
    let end = rest.find("</div>")?;
    Some(rest[..end].trim().to_string())
}

pub fn extract_img_src_by_class(block: &str, class_name: &str) -> Option<String> {
    let pos = block.find(class_name)?;
    let before = &block[..pos];
    let src_pos = before.rfind("src=\"")
        .or_else(|| block[pos..].find("src=\"").map(|p| pos + p))?;
    let start = src_pos + "src=\"".len();
    let rest = &block[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

pub fn extract_input_value(block: &str, name: &str) -> Option<String> {
    let needle_double = format!("name=\"{}\"", name);
    let needle_single = format!("name='{}'", name);
    let pos = block.find(&needle_double).or_else(|| block.find(&needle_single))?;
    let after = &block[pos..];
    if let Some(value_pos) = after.find("value=\"") {
        let start = pos + value_pos + "value=\"".len();
        let rest = &block[start..];
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    if let Some(value_pos) = after.find("value='") {
        let start = pos + value_pos + "value='".len();
        let rest = &block[start..];
        let end = rest.find('\'')?;
        return Some(rest[..end].to_string());
    }
    None
}
