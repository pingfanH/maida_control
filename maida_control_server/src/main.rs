mod mobile_handle;

use axum::{
    routing::get,
    Router,
    response::{IntoResponse, Redirect, Json},
    extract::Query,
    http::StatusCode,
};
use tower_http::cors::{CorsLayer, Any};

use serde::Serialize;
use serde::Deserialize;
use std::net::SocketAddr;
use axum::http::{header, HeaderMap};
use serde_json::{json, Value};
use libs::mai_api::get_user_preview_api;
use reqwest::redirect::Policy;
use anyhow::{anyhow, Result};
#[tokio::main]
async fn main() {
    tokio::spawn(proxy::service());
    // 路由配置
    let cors = CorsLayer::new()
        .allow_origin(Any) // 允许所有来源
        .allow_methods(Any) // 允许所有方法 GET/POST/PUT...
        .allow_headers(vec![
            "*".parse().unwrap()
        ]); // 允许自定义 headers
    let app = Router::new()
        .route("/", get(root))
        .route("/api", get(api))
        .route("/go", get(redirect_demo))
        .route("/oauth/authorize/maimai-dx", get(oauth_authorize))
        .layer(cors);

    // 绑定地址
    let addr = SocketAddr::from(([0, 0, 0, 0], 9855));
    println!("服务启动在 http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}

// 根路由
async fn root() -> &'static str {
    "MaiDaControl is Working!"
}

// 返回 JSON
#[derive(Serialize)]
struct HelloResponse {
    message: String,
}

async fn hello() -> impl IntoResponse {
    Json(HelloResponse {
        message: "Hello from Axum".to_string(),
    })
}

// 返回一个 302 重定向
async fn redirect_demo() -> Redirect {
    Redirect::temporary("https://www.rust-lang.org/")
}


    
    Json(json!({}))
}

#[derive(Deserialize)]
struct RedirectQuery {
    redirect_base: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    #[serde(rename = "errorID")]
    error_id: u32,
    #[serde(rename = "openGameID")]
    open_game_id: String,
    #[serde(rename = "userID")]
    user_id: u64,
    #[serde(rename = "sessionId")]
    session_id: u64,
    #[serde(rename = "userPlayFlag")]
    user_play_flag: bool,
    #[serde(rename = "newUserIdFlag")]
    new_user_id_flag: bool,
    #[serde(rename = "openGameIDFlag")]
    open_game_id_flag: bool,
}

async fn oauth_authorize(headers: HeaderMap, Query(query): Query<RedirectQuery>) -> impl IntoResponse {
    let ua = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("-");
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("-");
    println!("[oauth] incoming request ua={ua} origin={origin} redirect_base={:?}", query.redirect_base);
    match handle_oauth_authorize(query).await {
        Ok(redirect_url) => Redirect::temporary(&redirect_url).into_response(),
        Err(e) => {
            println!("[oauth] error: {e}");
            let body = format!("oauth authorize error: {e}");
            (StatusCode::BAD_GATEWAY, body).into_response()
        }
    }
}

async fn handle_oauth_authorize(query: RedirectQuery) -> Result<String> {
    let authorize_url = "https://tgk-wcaime.wahlap.com/wc_auth/oauth/authorize/maimai-dx";
    println!("[oauth] fetching authorize url: {authorize_url}");
    let location = get_open_url(authorize_url).await?;
    println!("[oauth] location: {location}");
    if location.contains("open.weixin.qq.com/connect/oauth2/authorize") {
        let rewritten = rewrite_wechat_redirect(&location);
        println!("[oauth] redirect to wechat (rewritten): {rewritten}");
        return Ok(rewritten);
    }

    let login = get_user_data_handle(location).await?;

    let base = query.redirect_base.unwrap_or_else(|| "https://127.0.0.1:5173".to_string());
    let base = base.trim_end_matches('/');
    let redirect_url = format!(
        "{}/home?user_id={}&open_game_id={}&session_id={}&user_play_flag={}&new_user_id_flag={}&open_game_id_flag={}",
        base,
        login.user_id,
        login.open_game_id,
        login.session_id,
        login.user_play_flag,
        login.new_user_id_flag,
        login.open_game_id_flag
    );
    Ok(redirect_url)
}

async fn get_open_url(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()?;
    let res = client.get(url).send().await?;
    println!("[oauth] authorize status: {}", res.status());
    let location = res.headers().get("location").ok_or_else(|| anyhow!("missing location header"))?;
    Ok(location.to_str()?.to_string())
}

async fn get_user_data_handle(url: String) -> Result<LoginResponse> {
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()?;
    println!("[oauth] login url: {url}");
    let res = client.get(&url).send().await?;
    let status = res.status();
    let text = res.text().await?;
    println!("[oauth] login status: {status} body_len={}", text.len());
    let parts: Vec<&str> = text.split("login=").collect();
    if parts.len() < 2 {
        let snippet: String = text.chars().take(300).collect();
        return Err(anyhow!("login payload not found in response. body_snippet={snippet}"));
    }
    let text = parts[1];
    let json_part = text
        .trim_end()
        .strip_suffix('"')
        .unwrap_or(text)
        .trim();
    let parsed: LoginResponse = serde_json::from_str(&json_part)?;
    Ok(parsed)
}

fn rewrite_wechat_redirect(url: &str) -> String {
    url.replace(
        "https%3A%2F%2Ftgk-wcaime.wahlap.com",
        "http%3A%2F%2Ftgk-wcaime.wahlap.com",
    )
}
