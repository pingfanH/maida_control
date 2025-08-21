use axum::{
    routing::{get},
    Router,
    response::{IntoResponse, Redirect, Json},
};
use tower_http::cors::{CorsLayer, Any};

use serde::Serialize;
use std::net::SocketAddr;
use axum::http::{header, HeaderMap};
use serde_json::{json, Value};
use libs::mai_api::get_user_preview_api;
#[tokio::main]
async fn main() {
    // 路由配置
    let cors = CorsLayer::new()
        .allow_origin(Any) // 允许所有来源
        .allow_methods(Any) // 允许所有方法 GET/POST/PUT...
        .allow_headers(vec![
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            "X-User-Id".parse().unwrap(),
            "X-Open-Game-Id".parse().unwrap(),
            "X-Session-Id".parse().unwrap()
        ]); // 允许自定义 headers
    let app = Router::new()
        .route("/", get(root))
        .route("/user_info", get(user_info))
        .route("/go", get(redirect_demo))
        .layer(cors);

    // 绑定地址
    let addr = SocketAddr::from(([0, 0, 0, 0], 9855));
    println!("🚀 服务启动在 http://{}", addr);
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

async fn user_info(headers: HeaderMap) -> Json<Value> {
    // 读取自定义 headers
    let user_id = headers.get("X-User-Id").and_then(|v| v.to_str().ok()).unwrap_or("");
    let open_game_id = headers.get("X-Open-Game-Id").and_then(|v| v.to_str().ok()).unwrap_or("");
    let session_id = headers.get("X-Session-Id").and_then(|v| v.to_str().ok()).unwrap_or("");
    let data = json!({
        "userId":user_id
    });
    let res = get_user_preview_api(data,user_id.to_string()).await;
    if let Ok(res) =res{
        println!("{:?}", res);
       return Json(serde_json::from_str(&res).unwrap());
    }else {
        println!("{:?}", res);
    }

    
    Json(json!({}))
}
