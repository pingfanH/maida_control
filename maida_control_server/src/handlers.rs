use axum::{
    routing::{get, post},
    response::{IntoResponse, Redirect, Json},
    http::StatusCode,
    Router,
};
use axum::extract::State;
use axum::http::HeaderMap;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

use crate::models::{AppState, FavoriteSetRequest};
use crate::services::{
    fetch_favorite_update_music_html,
    fetch_favorite_update_music_html_cached,
    fetch_favorites,
    fetch_local_favorites,
    fetch_music_list,
    fetch_session,
    get_session_expired,
    get_open_url,
    load_cached_html,
    load_session_by_user_id,
    refresh_favorites_cache,
    refresh_home_html,
    save_cache_html,
    set_session_expired,
    save_local_favorites,
    sync_favorites_from_remote_to_db,
    sync_favorites_with_remote,
    sync_music_data,
    HomeRefreshResult,
    extract_profile_from_html,
};

fn require_user_id(headers: &HeaderMap) -> Result<String, Value> {
    match headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        Some(id) => Ok(id),
        None => Err(json!({ "error": "missing x-user-id" })),
    }
}

// Build all HTTP routes in one place for easier discovery.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(vec!["*".parse().unwrap()]);
    Router::new()
        .route("/", get(root))
        .route("/api", get(api))
        .route("/api/favorites", post(set_local_favorites))
        .route("/api/favorites/sync", post(sync_favorites_remote))
        .route("/go", get(redirect_demo))
        .route("/oauth/authorize/maimai-dx", get(oauth_authorize))
        .with_state(state)
        .layer(cors)
}

// Liveness check.
async fn root() -> &'static str {
    "MaiDaControl is Working!"
}

// Debug endpoint kept for quick sanity checks.
#[allow(dead_code)]
async fn hello() -> impl IntoResponse {
    #[derive(serde::Serialize)]
    struct HelloResponse {
        message: String,
    }
    Json(HelloResponse {
        message: "Hello from Axum".to_string(),
    })
}

async fn redirect_demo() -> Redirect {
    Redirect::temporary("https://www.rust-lang.org/")
}

async fn oauth_authorize() -> impl IntoResponse {
    match get_open_url("https://tgk-wcaime.wahlap.com/wc_auth/oauth/authorize/maimai-dx").await {
        Ok(location) => {
            log::info!("[oauth] redirect -> {location}");
            Redirect::temporary(&location).into_response()
        }
        Err(e) => {
            let body = format!("oauth authorize error: {e}");
            (StatusCode::BAD_GATEWAY, body).into_response()
        }
    }
}

// Unified API gateway for frontend.
async fn api(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
   
    let get_header = |key: &str| -> Result<String, anyhow::Error> {
        headers
            .get(key)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned())
            .ok_or_else(|| anyhow::anyhow!("缺少 header: {}", key))
    };
    
    let method: String = match get_header("method") {
        Ok(m) => m,
        Err(e) => {
            log::info!("❌ 缺少 method: {:?}", e);
            return Json(json!({ "error": e.to_string() }));
        }
    };

    let user_id = match require_user_id(&headers) {
        Ok(id) => id,
        Err(err) => {
                println!("userid {err:?}");
            return Json(err)
        },
    };


    match method.as_str() {
        "Favorites" => match fetch_favorites(&state.pool, &user_id).await {
            Ok(val) => Json(val),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        "OwnHomeData" => {
            let mut value = json!({});
            if let Ok(Some(session)) = load_session_by_user_id(&state.pool, &user_id).await {
                let user_key = session.open_user_id.to_string();
                if let Ok(true) = get_session_expired(&state.pool, &user_key).await {
                    return Json(json!({
                        "redirect": "http://tgk-wcaime.wahlap.com/wc_auth/oauth/authorize/maimai-dx"
                    }));
                }
                let bg_pool = state.pool.clone();
                let bg_user = user_key.clone();
                tokio::spawn(async move {
                    match refresh_home_html(&bg_pool, &bg_user).await {
                        Ok(HomeRefreshResult::Expired) => {
                            let _ = set_session_expired(&bg_pool, &bg_user, true).await;
                        }
                        Ok(HomeRefreshResult::Ok(html)) => {
                            let _ = save_cache_html(&bg_pool, "mobile_html", &bg_user, &html).await;
                            let _ = set_session_expired(&bg_pool, &bg_user, false).await;
                            match sync_favorites_from_remote_to_db(&bg_pool, &user_id).await {
                                Err(err)=>{

                                    log::error!("[favorites-sync] background sync failed: {}", err);
                                    
                                },
                                _=>{}
                            }

                        }
                        Err(_) => {}
                    }
                });
                if let Ok(Some(html)) = load_cached_html(&state.pool, "mobile_html", &user_key).await {
                    let (title, nickname, rating) = extract_profile_from_html(&html);
                    if let Some(t) = title {
                        value["trophyTitle"] = json!(t);
                    }
                    if let Some(n) = nickname {
                        value["nickname"] = json!(n);
                        value["userName"] = json!(value["nickname"].as_str().unwrap_or("").to_string());
                    }
                    if let Some(r) = rating {
                        value["rating"] = json!(r);
                        value["playerRating"] = json!(value["rating"].clone());
                    }
                }
            } else {
                value["pending"] = json!(true);
            }
            Json(value)
        }
        "Session" => match fetch_session(&state.pool, &user_id).await {
            Ok(val) => Json(val),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        "SyncMusicData" => match sync_music_data(&state.pool).await {
            Ok(val) => Json(val),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        "FavoriteUpdateMusicHtml" => match fetch_favorite_update_music_html(&state.pool, &user_id).await {
            Ok(val) => Json(val),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        "FavoriteUpdateMusicHtmlCache" => match fetch_favorite_update_music_html_cached(&state.pool, &user_id).await {
            Ok(val) => Json(val),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        "MusicList" => match fetch_music_list(&state.pool).await {
            Ok(val) => Json(val),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        "FavoriteList" => match fetch_local_favorites(&state.pool, &user_id).await {
            Ok(val) => Json(val),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        "FavoritesSync" => match refresh_favorites_cache(&state.pool, &user_id).await {
            Ok(val) => Json(val),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        _ => Json(json!({ "error": "未知 method" })),
    }
}

async fn set_local_favorites(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<FavoriteSetRequest>,
) -> Json<Value> {
    let user_id = match require_user_id(&headers) {
        Ok(id) => id,
        Err(err) => return Json(err),
    };
    match save_local_favorites(&state.pool, &user_id, payload.song_ids).await {
        Ok(count) => Json(json!({ "saved": true, "count": count })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn sync_favorites_remote(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
    let user_id = match require_user_id(&headers) {
        Ok(id) => id,
        Err(err) => return Json(err),
    };
    let pool = state.pool.clone();
    let res = sync_favorites_with_remote(&pool, &user_id).await;
    match res {
        Ok(val) => Json(val),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
