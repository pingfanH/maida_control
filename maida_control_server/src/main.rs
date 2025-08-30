mod mobile_handle;

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
use mobile_handle::vo;
use crate::mobile_handle::get_records;
use crate::mobile_handle::vo::UserData;

const BASE_API: &str ="https://maimai.wahlap.com/maimai-mobile/";

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


async fn api(headers: HeaderMap) -> Json<Value> {
    // 小工具：从 header 里安全取值
    let get_header = |key: &str| -> Result<String, anyhow::Error> {
        headers
            .get(key)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned())
            .ok_or_else(|| anyhow::anyhow!("缺少 header: {}", key))
    };

    // 构建 UserData

    let user_data = match (
        get_header("X-User-Id"),
        get_header("X-Open-User-Id"),
        get_header("X-Session-Id"),
    ) {
        (Ok(user_id), Ok(open_user_id), Ok(session_id)) => UserData {
            user_id,
            open_user_id,
            session_id,
        },
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
            return Json(json!({ "error": e.to_string() }));
        }
    };


    let method = match get_header("method") {
        Ok(m) => m,
        Err(e) => {
            println!("❌ 缺少 method: {:?}", e);
            return Json(json!({ "error": e.to_string() }));
        }
    };

    let data = json!({
        "userId": user_data.user_id
    });

    match method.as_str() {
        "Favorites" => {
            // TODO: Favorites 逻辑
            Json(json!({ "msg": "Favorites 未实现" }))
        }
        "OwnHomeData" => {
            match get_user_preview_api(data, user_data.user_id.clone()).await {
                Ok(res) => {
                    println!("{:?}", res);
                    get_records(user_data).await;
                    // 这里直接转成 Json 返回
                    Json(serde_json::from_str(&res).unwrap_or(json!({ "error": "解析失败" })))
                }
                Err(e) => {
                    println!("❌ OwnHomeData 请求失败: {:?}", e);
                    Json(json!({ "error": e.to_string() }))
                }
            }
        }
        _ => Json(json!({ "error": "未知 method" })),
    }
}