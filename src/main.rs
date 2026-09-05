//! rustway 服务入口。
//!
//! 用法：
//! - `rustway serve`    启动本地服务（默认；axum 同时提供 API 与前端静态站点）
//! - `rustway validate` 校验内容并退出（exit 0 = 通过，1 = 有错误）；CI/内容贡献者用
//!
//! 环境变量：
//! - RUSTWAY_CONTENT 内容目录（默认 ./content）
//! - RUSTWAY_STATIC  静态前端目录（默认 ./frontend/dist）
//! - RUSTWAY_PORT    监听端口（默认 8080）
//!
//! 安全约定：本程序不发起出站请求、不读取任何凭据；仅监听本地地址。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use axum::Router;
use rustway::Store;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;

fn content_dir() -> PathBuf {
    PathBuf::from(std::env::var("RUSTWAY_CONTENT").unwrap_or_else(|_| "content".into()))
}

fn load_or_exit() -> Store {
    let dir = content_dir();
    match rustway::load_store(&dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("✗ 内容加载失败: {e}");
            std::process::exit(1);
        }
    }
}

fn report(store: &Store) -> bool {
    if !store.errors.is_empty() {
        eprintln!("✗ 内容校验失败（{} 个错误）：", store.errors.len());
        for e in &store.errors {
            eprintln!("  ✗ {e}");
        }
    }
    if !store.warnings.is_empty() {
        eprintln!("⚠ {} 个警告（不阻塞，建议修复）：", store.warnings.len());
        for w in store.warnings.iter().take(30) {
            eprintln!("  ⚠ {w}");
        }
        if store.warnings.len() > 30 {
            eprintln!("  … 以及另外 {} 个", store.warnings.len() - 30);
        }
    }
    if store.errors.is_empty() {
        println!(
            "✓ 内容校验通过：{} 模块 / {} 章节 / {} 知识点 / {} 测验题 / {} 待编排 / {} 警告",
            store.curriculum.modules.len(),
            store
                .curriculum
                .modules
                .iter()
                .map(|m| m.chapters.len())
                .sum::<usize>(),
            store.kps.len(),
            store.total_quiz(),
            store.unplaced.len(),
            store.warnings.len(),
        );
        true
    } else {
        false
    }
}

#[tokio::main]
async fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| "serve".into());
    match cmd.as_str() {
        "validate" => {
            let store = load_or_exit();
            std::process::exit(if report(&store) { 0 } else { 1 });
        }
        "serve" => serve().await,
        other => {
            eprintln!("未知子命令: {other}（可用: serve | validate）");
            std::process::exit(2);
        }
    }
}

async fn serve() {
    let store = load_or_exit();
    let ok = report(&store);
    if !ok {
        eprintln!("（内容存在错误，仍可启动预览，但 cargo test / CI 会失败）");
    }

    println!("════════════════════════════════════════");
    println!("  {}", store.curriculum.title);
    println!("  {}", store.curriculum.subtitle);
    println!("════════════════════════════════════════");

    let state = Arc::new(store);
    let static_dir = std::env::var("RUSTWAY_STATIC").unwrap_or_else(|_| "frontend/dist".into());
    let app = Router::new()
        .route("/api/manifest", get(manifest))
        .route("/api/kp/{id}", get(kp_detail))
        .route("/api/health", get(health))
        .fallback_service(ServeDir::new(static_dir).append_index_html_on_directories(true))
        .with_state(state);

    let port: u16 = std::env::var("RUSTWAY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("无法绑定 {addr}: {e}"));
    println!("✓ 服务已启动: http://{addr}（Ctrl+C 优雅退出）");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            println!("\n正在关闭…");
        })
        .await
        .unwrap();
}

async fn manifest(State(st): State<Arc<Store>>) -> Json<serde_json::Value> {
    Json(st.manifest_json())
}

async fn kp_detail(
    State(st): State<Arc<Store>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    st.kp_json(&id).map(Json).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "kp_not_found",
                "id": id,
                "message": "知识点不存在",
            })),
        )
    })
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
