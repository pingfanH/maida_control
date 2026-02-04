use std::collections::HashMap;
use std::env;
use std::sync::OnceLock;
use httparse::Header;
use reqwest::redirect::Policy;
use reqwest::Url;
use anyhow::{anyhow, Error, Result};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

use serde::Deserialize;

fn debug_enabled() -> bool {
    matches!(
        env::var("DEBUG_LOG").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

#[derive(Debug, serde::Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "errorID")]
    pub error_id: u32,

    #[serde(rename = "openGameID")]
    pub open_game_id: String,

    #[serde(rename = "userID")]
    pub user_id: u64,

    #[serde(rename = "sessionId")]
    pub session_id: u64,

    #[serde(rename = "userPlayFlag")]
    pub user_play_flag: bool,

    #[serde(rename = "newUserIdFlag")]
    pub new_user_id_flag: bool,

    #[serde(rename = "openGameIDFlag")]
    pub open_game_id_flag: bool,
}

static DB_POOL: OnceLock<MySqlPool> = OnceLock::new();

async fn db_pool() -> Result<MySqlPool> {
    dotenvy::dotenv().ok();
    if let Some(pool) = DB_POOL.get() {
        return Ok(pool.clone());
    }
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| anyhow!("DATABASE_URL is not set"))?;
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let _ = DB_POOL.set(pool.clone());
    Ok(pool)
}

async fn upsert_session(pool: &MySqlPool, login: &LoginResponse, user_key: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO maimai_session (user_id, open_user_id, session_id, open_game_id, user_play_flag, new_user_id_flag, open_game_id_flag)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE
            open_user_id = VALUES(open_user_id),
            session_id = VALUES(session_id),
            open_game_id = VALUES(open_game_id),
            user_play_flag = VALUES(user_play_flag),
            new_user_id_flag = VALUES(new_user_id_flag),
            open_game_id_flag = VALUES(open_game_id_flag)",
    )
    .bind(login.user_id as i64)
    .bind(user_key)
    .bind(login.session_id as i64)
    .bind(&login.open_game_id)
    .bind(login.user_play_flag)
    .bind(login.new_user_id_flag)
    .bind(login.open_game_id_flag)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_cookies(pool: &MySqlPool, user_key: &str, cookies: &HashMap<String, String>) -> Result<()> {
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

async fn upsert_cache(pool: &MySqlPool, cache_key: &str, user_key: &str, content: &str) -> Result<()> {
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

///open.weixin.qq.com
pub async fn get_open_url(url:&String)->Result<String>{
    let client = reqwest::Client::builder()
        .redirect(Policy::none()) // <-- 关键！禁止自动重定向
        .build()?;
    let method = reqwest::Method::GET;

    let mut req_builder = client.request(method, url);
    match req_builder.send().await {
        Ok(res)=>{
          //  log::info!("\n✅ 成功从目标服务器获取响应 ✅");
          //  log::info!("<-- 响应状态: {}", res.status());
            let location = res.headers().get("location");
            if let Some(_location)=location{
                let _location = _location.to_str()?.to_string();
                let _location = _location.replace("https%3A%2F%2Ftgk-wcaime.wahlap.com","http%3A%2F%2Ftgk-wcaime.wahlap.com");
                if debug_enabled() {    
                if debug_enabled() {
                    log::info!("location:{}", _location);
                }
                }

                Ok(_location)
            }else {
                Err(anyhow!("在响应中未找到 Location 头部"))
            }
        },
        Err(e) => Err(Error::from(e)),
    }

}

///URL tgk-wcaime.wahlap.com/wc_auth/oauth/callback/maimai-dx?r=___&t=___&code=___&state=___
pub async fn maimai_handle<'headers, 'buf>(full_url:String, headers: &'headers mut [Header<'buf>])->Result<(LoginResponse,String)>{

        if debug_enabled() {
            if debug_enabled() {
                log::info!("\n✅ 成功捕获请求，准备使用 reqwest 转发 ✅");
                log::info!("URL: {}", full_url);
            }
        }

        let client = reqwest::Client::builder()
            .redirect(Policy::none()) // <-- 关键！禁止自动重定向
            .build()?;
        let method = reqwest::Method::GET;

        let mut req_builder = client.request(method, &full_url);
        if debug_enabled() {
            if debug_enabled() {
                log::info!("--> 正在转发其余 headers:");
            }
        }
        for header in headers.iter() {
            if !header.name.eq_ignore_ascii_case("Host") && !header.name.eq_ignore_ascii_case("Proxy-Connection") {
                if debug_enabled() {
                    if debug_enabled() {
                        log::info!("    {}: {}", header.name, std::str::from_utf8(header.value).unwrap_or("<invalid utf8>"));
                    }
                }
                req_builder = req_builder.header(header.name, header.value);
            }
        }

        match req_builder.send().await {
            Ok(response) => {
                if debug_enabled() {
                    if debug_enabled() {
                        log::info!("\n✅ 成功从目标服务器获取data响应 ✅");
                        log::info!("URL: {}", full_url);
                        log::info!("<-- 响应状态: {}", response.status());
                    }
                }

                let location = response.headers().get("location");
                if let Some(_location) = location {
                    if debug_enabled() {
                        if debug_enabled() {
                            log::info!("location:{}", _location.to_str()?.to_string());
                        }
                    }
                    let (res,_cookies) = get_user_data_handle(_location.to_str()?.to_string()).await?;
                    let user_key = res.user_id.to_string();
                    Ok((res,user_key))
                } else {
                    Err(anyhow!("在响应中未找到 Location 头部"))
                }


            },
            Err(e) => Err(Error::from(e)),
        }
}

///URL maimai.wahlap.com/maimai-mobile/?t=___
pub async fn get_user_data_handle(url:String)-> Result<(LoginResponse,HashMap<String, String>)>{
    let client = reqwest::Client::builder()
        .redirect(Policy::none()) // <-- 关键！禁止自动重定向
        .build()?;
    let method = reqwest::Method::GET;

    let parsed_url = Url::parse(&url)?;
    let t_param = parsed_url
        .query_pairs()
        .find(|(k, _)| k == "t")
        .map(|(_, v)| v.to_string());

    let mut req_builder = client.request(method, &url);
    match req_builder.send().await {
        Ok(res)=>{
            if debug_enabled() {
                if debug_enabled() {
                    log::info!("\n✅ 成功从目标服务器获取响应 ✅");
                    log::info!("<-- 响应状态: {}", res.status());
                }
            }

            let mut cookies = HashMap::new();
            for val in res.headers().get_all("set-cookie").iter() {
                if let Ok(s) = val.to_str() {
                    if let Some((k, v)) = s.split_once('=') {
                        // cookie 只取到第一个分号前
                        let v = v.split(';').next().unwrap_or("").to_string();
                        cookies.insert(k.trim().to_string(), v);
                    }
                }
            }
            let location = res.headers().get("location");
                if let Some(_location) = location {
                    if debug_enabled() {
                        if debug_enabled() {
                            log::info!("location:{}", _location.to_str()?.to_string());
                        }
                    }
    
                } else {
                if debug_enabled() {
                    log::info!("在响应中未找到 Location 头部");
                }
                }


            let text = res.text().await?;
            let text = {let texts:Vec<&str>=text.split("login=").collect();texts[1]};
            let json_part = text
                .trim_end()                // 去掉 \n \r 空格
                .strip_suffix('"')         // 去掉最后一个 "
                .unwrap_or(text)
                .trim();                   // 再保险修剪一次

            //log::info!("json_part:{json_part}");
            let parsed: LoginResponse = serde_json::from_str(&json_part)?;


            // 使用刚拿到的 cookies 访问 maimai-mobile 并保存 HTML
            let cookie_header = cookies
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            let mobile_url = if let Some(t) = t_param {
                format!("https://maimai.wahlap.com/maimai-mobile/home/?t={}", t)
            } else {
                "https://maimai.wahlap.com/maimai-mobile/home/".to_string()
            };
            let html = client
                .get(&mobile_url)
                .header("Cookie", cookie_header)
                .header("User-Agent", "Mozilla/5.0")
                .header("Referer", "https://maimai.wahlap.com/maimai-mobile/")
                .send()
                .await?
                .text()
                .await?;
            let user_key = parsed.user_id.to_string();
            if !user_key.is_empty() {
                let pool = db_pool().await?;
                upsert_session(&pool, &parsed, &user_key).await?;
                upsert_cookies(&pool, &user_key, &cookies).await?;
                upsert_cache(&pool, "mobile_html", &user_key, &html).await?;
                // Clear expired flag after successful login.
                upsert_cache(&pool, "session_expired", &user_key, "0").await?;
            }

            Ok((parsed,cookies))
        },
        Err(e) => Err(Error::from(e)),
    }
}
