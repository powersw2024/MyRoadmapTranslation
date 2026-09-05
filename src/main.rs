//! rustway 服务入口。
//!
//! 用法：
//! - `rustway serve`            启动本地服务（API + 前端静态站点 + SQLite 映射库一主一备）
//! - `rustway validate`         校验内容并退出（exit 0 = 通过）；CI 与贡献者用
//! - `rustway import <file>`    校验一个外部提交的知识点 JSON（按 schema/kp-file.schema.json 收集），
//!   在沙箱中报告与现有内容的冲突/缺口；通过后再由人工放入 content/kps/
//! - `rustway schema`           打印知识点 JSON Schema（发布给内容贡献者）
//!
//! 环境变量：
//! - RUSTWAY_CONTENT 内容目录（默认 ./content）
//! - RUSTWAY_STATIC  静态前端目录（默认 ./frontend/dist）
//! - RUSTWAY_DATA    SQLite 数据目录（默认 ./data；主库 rustway.db + 备库 rustway-backup.db）
//! - RUSTWAY_PORT    监听端口（默认 8080）
//!
//! 安全约定：本程序不发起出站请求、不读取任何凭据；SQL 全部参数绑定。

use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Json;
use axum::routing::get;
use axum::Router;
use rustway::db::DbSnapshot;
use rustway::Store;
use serde_json::json;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// 服务状态：内容源（内存）+ 结构映射库快照（SQLite 回读）。
#[derive(Clone)]
struct AppState {
    store: Arc<Store>,
    db: Option<Arc<DbSnapshot>>,
}

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
            "✓ 内容校验通过：{} 模块 / {} 章节 / {} 知识点 / {} 测验题 / {} 待编排 / {} 相似边 / {} 警告",
            store.curriculum.modules.len(),
            store.curriculum.modules.iter().map(|m| m.chapters.len()).sum::<usize>(),
            store.kps.len(),
            store.total_quiz(),
            store.unplaced.len(),
            store.similar_edges.len(),
            store.warnings.len(),
        );
        true
    } else {
        false
    }
}

/// `rustway import <file>`：把外部提交的 JSON 放进「内容沙箱」做全量校验，
/// 不改动正式 content/；通过则提示人工放置路径。
fn import(file: &str) -> i32 {
    let src = PathBuf::from(file);
    if !src.exists() {
        eprintln!("✗ 文件不存在: {}", src.display());
        return 2;
    }
    let tmp = std::env::temp_dir().join(format!("rustway-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let copy_kps = |from: &StdPath, to: &StdPath| -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let e = entry?;
            if e.path().extension().and_then(|s| s.to_str()) == Some("json") {
                std::fs::copy(e.path(), to.join(e.file_name()))?;
            }
        }
        Ok(())
    };
    if let Err(e) = copy_kps(&content_dir().join("kps"), &tmp.join("kps")) {
        eprintln!("✗ 组装沙箱失败: {e}");
        return 2;
    }
    if let Err(e) = std::fs::copy(
        content_dir().join("curriculum.json"),
        tmp.join("curriculum.json"),
    ) {
        eprintln!("✗ 组装沙箱失败: {e}");
        return 2;
    }
    let dest = tmp.join("kps").join(
        src.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("imported.json")),
    );
    if let Err(e) = std::fs::copy(&src, &dest) {
        eprintln!("✗ 复制提交文件失败: {e}");
        return 2;
    }
    println!("⊙ 沙箱校验 {} …", src.display());
    match rustway::load_store(&tmp) {
        Ok(store) => {
            let ok = report(&store);
            let _ = std::fs::remove_dir_all(&tmp);
            if ok {
                println!(
                    "✓ 提交文件可入库。放置路径：content/kps/{}（编入章节请在 curriculum.json 对应章节的 kps 数组挂载 id）",
                    dest.file_name().unwrap_or_default().to_string_lossy()
                );
                0
            } else {
                1
            }
        }
        Err(e) => {
            eprintln!("✗ 校验失败: {e}");
            let _ = std::fs::remove_dir_all(&tmp);
            1
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()).unwrap_or("serve") {
        "validate" => {
            let store = load_or_exit();
            std::process::exit(if report(&store) { 0 } else { 1 });
        }
        "schema" => {
            println!("{}", rustway::KP_FILE_SCHEMA);
        }
        "import" => match args.get(2) {
            Some(file) => std::process::exit(import(file)),
            None => {
                eprintln!("用法: rustway import <file.json>");
                std::process::exit(2);
            }
        },
        "serve" => serve().await,
        other => {
            eprintln!("未知子命令: {other}（可用: serve | validate | import <file> | schema）");
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

    // SQLite 映射库：一主一备。失败不阻塞服务（健康检查如实上报），可用性优先。
    let data_dir = PathBuf::from(std::env::var("RUSTWAY_DATA").unwrap_or_else(|_| "data".into()));
    let db = match rustway::db::open_sync_and_backup(&store, &data_dir) {
        Ok(snap) => {
            println!(
                "✓ 映射库已同步：{} 条编排 / {} 条前置 / {} 条相似 → 主库 {} + 备库 {}",
                snap.placements.len(),
                snap.prereq_edges.len(),
                snap.similar_edges.len(),
                snap.primary_path.display(),
                snap.backup_path.display()
            );
            Some(Arc::new(snap))
        }
        Err(e) => {
            eprintln!("⚠ 映射库不可用（服务继续，健康检查将上报）: {e}");
            None
        }
    };

    println!("════════════════════════════════════════");
    println!("  {}", store.curriculum.title);
    println!("  {}", store.curriculum.subtitle);
    println!("════════════════════════════════════════");

    let state = AppState {
        store: Arc::new(store),
        db,
    };
    let static_dir = std::env::var("RUSTWAY_STATIC").unwrap_or_else(|_| "frontend/dist".into());
    // 静态资源带缓存头（Vite 产物含内容哈希，可放心缓存）
    let static_service = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=3600"),
        ))
        .service(ServeDir::new(static_dir).append_index_html_on_directories(true));

    let app = Router::new()
        .route("/api/manifest", get(manifest))
        .route("/api/kp/{id}", get(kp_detail))
        .route("/api/health", get(health))
        .route("/api/schema", get(schema))
        .fallback_service(static_service)
        .layer(CompressionLayer::new()) // gzip/br：manifest 与 dist 资源瘦身
        .layer(TraceLayer::new_for_http()) // 请求日志 → tracing 输出
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

async fn manifest(State(st): State<AppState>) -> Json<serde_json::Value> {
    Json(st.store.manifest_json())
}

async fn kp_detail(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    st.store.kp_json(&id).map(Json).ok_or_else(|| {
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

async fn health(State(st): State<AppState>) -> Json<serde_json::Value> {
    let db = st.db.as_ref().map(|d| {
        json!({
            "engine": "sqlite",
            "mode": "primary+backup",
            "primary": d.primary_path.display().to_string(),
            "backup": d.backup_path.display().to_string(),
            "syncedAt": d.synced_at,
            "placements": d.placements.len(),
            "prereqEdges": d.prereq_edges.len(),
            "similarEdges": d.similar_edges.len(),
        })
    });
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "db": db,
    }))
}

/// 知识点加载规范（对外发布）：贡献者按此结构编写 JSON 提交。
async fn schema() -> Json<serde_json::Value> {
    Json(json!({
        "file": serde_json::from_str::<serde_json::Value>(rustway::KP_FILE_SCHEMA).expect("kp-file schema 合法"),
        "kp": serde_json::from_str::<serde_json::Value>(rustway::KP_SCHEMA).expect("kp schema 合法"),
    }))
}
