mod handlers;
mod logger;
mod models;
mod services;

use std::env;
use std::net::SocketAddr;
use std::time::Duration;

use sqlx::mysql::MySqlPoolOptions;

use crate::handlers::build_router;
use crate::logger::init_logger;
use crate::models::AppState;
use crate::services::sync_music_data;
mod test;
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let _ = init_logger();
    tokio::spawn(proxy::service());

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL is not set");
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect database");
    let state = AppState { pool };

    // Periodic music data sync.
    let sync_interval_secs = env::var("MUSIC_SYNC_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(86400);
    if sync_interval_secs > 0 {
        let sync_pool = state.pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(sync_interval_secs));
            loop {
                interval.tick().await;
                if let Err(e) = sync_music_data(&sync_pool).await {
                    log::error!("sync_music_data failed: {e}");
                } else {
                    log::info!("sync_music_data success");
                }
            }
        });
    }

    let app = build_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 9855));
    log::info!("服务启动在 http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}
