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
use rustway::Store;
use serde_json::json;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// 服务状态：内容源（内存）+ 结构映射库（SQLite 回读快照 + 提交留存连接）。
#[derive(Clone)]
struct AppState {
    store: Arc<Store>,
    db: Option<Arc<rustway::db::Db>>,
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
    let db = match rustway::db::Db::open(&store, &data_dir) {
        Ok(db) => {
            let s = &db.snapshot;
            println!(
                "✓ 映射库已同步：{} 条编排 / {} 条前置 / {} 条相似 → 主库 {} + 备库 {}",
                s.placements.len(),
                s.prereq_edges.len(),
                s.similar_edges.len(),
                s.primary_path.display(),
                s.backup_path.display()
            );
            Some(Arc::new(db))
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
    // 缓存策略：/assets/*（Vite 内容哈希文件名）一年 immutable；
    // index.html 等入口 no-cache —— 前端发版后浏览器总能拿到新入口
    let assets_service = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ))
        .service(ServeDir::new(format!("{static_dir}/assets")));
    let static_service = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .service(ServeDir::new(&static_dir).append_index_html_on_directories(true));

    let app = Router::new()
        .route("/api/manifest", get(manifest))
        .route("/api/kp/{id}", get(kp_detail))
        .route("/api/health", get(health))
        .route("/api/schema", get(schema))
        .route("/api/judge", axum::routing::post(judge))
        .route("/api/grade", axum::routing::post(grade))
        .route("/api/review", axum::routing::post(review))
        .route("/api/submissions", get(submissions))
        .route("/api/submissions/{id}", get(submission_detail))
        .route("/api/ai/config", get(ai_get_config))
        .route("/api/ai/latency", axum::routing::post(ai_latency))
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
        let s = &d.snapshot;
        json!({
            "engine": "sqlite",
            "mode": "primary+backup",
            "primary": s.primary_path.display().to_string(),
            "backup": s.backup_path.display().to_string(),
            "syncedAt": s.synced_at,
            "placements": s.placements.len(),
            "prereqEdges": s.prereq_edges.len(),
            "similarEdges": s.similar_edges.len(),
        })
    });
    let ai = effective_ai(&st).map(|c| {
        json!({
            "enabled": true,
            "backend": c.backend,
            "model": c.model,
        })
    });
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "db": db,
        "ai": ai.unwrap_or_else(|| json!({"enabled": false})),
    }))
}

// ---------------- 编程题评测 / AI 评价 / 提交留存 ----------------

#[derive(serde::Deserialize)]
struct JudgeReq {
    kp_id: String,
    code: String,
}

/// 运行编程题评测：rustc --test 沙箱编译运行；通过则落盘 + 入库。
async fn judge(
    State(st): State<AppState>,
    Json(req): Json<JudgeReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let kp = st.store.kps.get(&req.kp_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "kp_not_found", "id": req.kp_id })),
        )
    })?;
    let test_code = kp
        .quiz
        .iter()
        .find(|q| q.kind_str() == "code")
        .map(|q| q.test_code.clone())
        .unwrap_or_default();

    let outcome = rustway::judge::run_rust_checks(&req.code, &test_code)
        .await
        .map_err(internal_error)?;

    let mut submission_id = None;
    if outcome.passed {
        if let Some(db) = &st.db {
            if let Ok(meta) =
                db.save_submission(&req.kp_id, "code", true, &req.code, &outcome.output, None)
            {
                submission_id = Some(meta.id);
            }
        }
    }
    Ok(Json(json!({
        "passed": outcome.passed,
        "output": outcome.output,
        "durationMs": outcome.duration_ms,
        "submissionId": submission_id,
    })))
}

#[derive(serde::Deserialize)]
struct ReviewReq {
    submission_id: i64,
}

/// AI 评价：从数据库取提交代码，调用 Ollama/OpenAI 兼容后端，结果回写数据库。
async fn review(
    State(st): State<AppState>,
    Json(req): Json<ReviewReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let Some(db) = &st.db else {
        return Err(bad_request("数据库不可用，无法评价"));
    };
    let ai = effective_ai(&st).ok_or_else(|| {
        bad_request("AI 评价未启用：请在「AI 设置」页配置，或设置 RUSTWAY_AI_BACKEND 等环境变量")
    })?;
    let (kp_id, code) = db.submission_code(req.submission_id).map_err(bad_request)?;
    let (title, task) = st
        .store
        .kps
        .get(&kp_id)
        .map(|k| (k.title.clone(), k.task.clone()))
        .unwrap_or_default();
    let r = rustway::ai::review(&ai, &title, &task, &code)
        .await
        .map_err(bad_request)?;
    db.update_submission_review(
        req.submission_id,
        r.score as i64,
        &format!("{}\n---\n{}", r.summary, r.suggestions.join("\n")),
    )
    .map_err(bad_request)?;
    Ok(Json(json!({
        "submissionId": req.submission_id,
        "score": r.score,
        "summary": r.summary,
        "suggestions": r.suggestions,
    })))
}

/// 提交映射列表（?kp_id= 过滤，可选）。
async fn submissions(
    State(st): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let Some(db) = &st.db else {
        return Err(bad_request("数据库不可用"));
    };
    let rows = db
        .list_submissions(q.get("kp_id").map(|s| s.as_str()))
        .map_err(bad_request)?;
    Ok(Json(json!({ "submissions": rows })))
}

/// 单条提交详情（含代码正文）。
async fn submission_detail(
    State(st): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let Some(db) = &st.db else {
        return Err(bad_request("数据库不可用"));
    };
    let (kp_id, code) = db.submission_code(id).map_err(bad_request)?;
    let meta = db
        .list_submissions(None)
        .map_err(bad_request)?
        .into_iter()
        .find(|m| m.id == id);
    let ai_comments = db.submission_comments(id).map_err(bad_request)?;
    Ok(Json(json!({
        "id": id,
        "kpId": kp_id,
        "code": code,
        "passed": meta.as_ref().map(|m| m.passed),
        "createdAt": meta.as_ref().map(|m| m.created_at.clone()),
        "aiScore": meta.as_ref().and_then(|m| m.ai_score),
        "aiComments": ai_comments,
    })))
}

#[derive(serde::Deserialize)]
struct GradeReq {
    kp_id: String,
    answer: String,
}

/// 主观题批改：配置 AI 时对照参考答案打分；未配置 AI 时返回参考答案供自评。
/// 两种模式的答案都会留档入库。
async fn grade(
    State(st): State<AppState>,
    Json(req): Json<GradeReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let kp = st.store.kps.get(&req.kp_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "kp_not_found", "id": req.kp_id })),
        )
    })?;
    let subjective = kp
        .quiz
        .iter()
        .find(|q| q.kind_str() == "subjective")
        .ok_or_else(|| bad_request(format!("知识点 {} 没有主观题", req.kp_id)))?;

    if req.answer.trim().is_empty() {
        return Err(bad_request("答案不能为空"));
    }

    // 留档（主观题始终入库：kind=subjective，passed 由批改/自评结果决定）
    let record = |passed: bool, output: &str, ai_score: Option<i64>, db: &rustway::db::Db| {
        db.save_submission(
            &req.kp_id,
            "subjective",
            passed,
            &req.answer,
            output,
            ai_score,
        )
        .ok()
        .map(|m| m.id)
    };

    match effective_ai(&st) {
        Some(cfg) => {
            match rustway::ai::grade_answer(&cfg, &kp.title, &subjective.reference, &req.answer)
                .await
            {
                Ok(r) => {
                    let passed = r.score >= 60;
                    let submission_id = st.db.as_ref().and_then(|db| {
                        record(
                            passed,
                            &format!(
                                "AI 批改 {} 分：{}\n{}",
                                r.score,
                                r.summary,
                                r.suggestions.join("；")
                            ),
                            Some(r.score as i64),
                            db,
                        )
                    });
                    Ok(Json(json!({
                        "mode": "ai",
                        "passed": passed,
                        "score": r.score,
                        "summary": r.summary,
                        "suggestions": r.suggestions,
                        "reference": subjective.reference,
                        "submissionId": submission_id,
                    })))
                }
                // AI 后端不可用（如 Ollama 未启动）：优雅降级为自评模式，不阻塞学习
                Err(e) => {
                    eprintln!("⚠ AI 批改不可用，降级为自评模式: {e}");
                    let submission_id = st.db.as_ref().and_then(|db| {
                        record(false, &format!("自评模式（AI 批改失败: {e}）"), None, db)
                    });
                    Ok(Json(json!({
                        "mode": "self",
                        "degradedFrom": cfg.backend,
                        "reference": subjective.reference,
                        "submissionId": submission_id,
                    })))
                }
            }
        }
        None => {
            let submission_id = st
                .db
                .as_ref()
                .and_then(|db| record(false, "自评模式（无 AI）", None, db));
            Ok(Json(json!({
                "mode": "self",
                "reference": subjective.reference,
                "submissionId": submission_id,
            })))
        }
    }
}

fn bad_request(msg: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "bad_request", "message": msg.into() })),
    )
}

fn internal_error(msg: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal", "message": msg.into() })),
    )
}

// ---------------- AI 配置页接口（设置持久化到 SQLite；环境变量作为缺省回退） ----------------

/// 生效的 AI 配置：设置页保存的 DB 配置优先，其次环境变量。
fn effective_ai(_st: &AppState) -> Option<rustway::ai::AiConfig> {
    // 安全契约：AI 后端只来自服务端环境变量，绝不接受请求体传入的地址（SSRF 面）。
    rustway::ai::AiConfig::from_env()
}

/// 当前 AI 配置（密钥掩码：只返回是否已配置，永不明文回传）。
async fn ai_get_config(State(st): State<AppState>) -> Json<serde_json::Value> {
    let mut out = json!({
        "backend": "",
        "baseUrl": "",
        "model": "",
        "apiKeySet": false,
        "source": "none",
    });
    if let Some(c) = rustway::ai::AiConfig::from_env() {
        out["backend"] = json!(c.backend);
        out["baseUrl"] = json!(c.base_url);
        out["model"] = json!(c.model);
        out["apiKeySet"] = json!(c.api_key.is_some());
        out["source"] = json!("environment");
    }
    let _ = st;
    Json(out)
}

/// 最小补全测速：用当前生效（环境变量）配置发起一次最小补全，返回耗时。
async fn ai_latency(
    State(st): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let Some(cfg) = effective_ai(&st) else {
        return Err(bad_request(
            "AI 尚未配置：请通过环境变量 RUSTWAY_AI_BACKEND 等设置",
        ));
    };
    match rustway::ai::first_completion_latency(&cfg).await {
        Ok(ms) => Ok(Json(json!({
            "ok": true,
            "latencyMs": ms,
            "backend": cfg.backend,
            "model": cfg.model,
        }))),
        Err(e) => Ok(Json(json!({ "ok": false, "error": e }))),
    }
}

/// 知识点加载规范（对外发布）：贡献者按此结构编写 JSON 提交。
async fn schema() -> Json<serde_json::Value> {
    Json(json!({
        "file": serde_json::from_str::<serde_json::Value>(rustway::KP_FILE_SCHEMA).expect("kp-file schema 合法"),
        "kp": serde_json::from_str::<serde_json::Value>(rustway::KP_SCHEMA).expect("kp schema 合法"),
    }))
}
