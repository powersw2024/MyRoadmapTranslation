//! 内容完整性集成测试 —— 本教程「可验证」承诺的强制执行者。
//!
//! 这些测试保证：
//! 1. 章节引用的知识点全部存在，无重复编排
//! 2. 每个知识点都有：摘要、验证任务、至少一道自测题、至少一条 http(s) 引用
//! 3. 前置依赖全部存在且无环
//! 4. 被引用的详细文档（detail_md）真实存在
//! 5. 课程骨架规模达标（防止骨架被意外删减）

use rustway::load_store;
use std::path::Path;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn real_store() -> rustway::Store {
    load_store(&repo_root().join("content")).expect("内容目录加载失败")
}

#[test]
fn real_content_passes_all_integrity_rules() {
    let store = real_store();
    assert!(
        store.errors.is_empty(),
        "内容完整性校验失败（共 {} 条）:\n  - {}",
        store.errors.len(),
        store.errors.join("\n  - ")
    );
}

#[test]
fn skeleton_scale_is_intact() {
    let store = real_store();
    assert!(store.curriculum.modules.len() >= 10, "模块数不足");
    assert!(
        store.kps.len() >= 170,
        "知识点数不足（当前 {}）",
        store.kps.len()
    );
    assert!(
        store.total_quiz() >= 170,
        "测验题总量不足（当前 {}）",
        store.total_quiz()
    );
    // 待编排知识点是特性不是缺陷，但不应失控
    assert!(
        store.unplaced.len() <= 10,
        "待编排知识点过多: {:?}",
        store.unplaced
    );
    // 学习方法模块必须是第 0 课
    assert_eq!(store.curriculum.modules[0].num, 0);
    assert!(
        store.curriculum.modules[0].title.contains("学习方法"),
        "第一个模块必须是学习方法论"
    );
}

#[test]
fn every_module_has_chapters_and_kps() {
    let store = real_store();
    for m in &store.curriculum.modules {
        assert!(!m.chapters.is_empty(), "模块 {} 没有章节", m.id);
        for c in &m.chapters {
            assert!(!c.kps.is_empty(), "章节 {} 没有知识点", c.id);
            assert!(!c.checkpoint.is_empty(), "章节 {} 缺少关卡测验", c.id);
        }
    }
}

#[test]
fn knowledge_graph_is_connected_enough() {
    // 图谱的价值在于前置关系：大多数知识点应当有前置（入门模块除外）
    let store = real_store();
    let placed: Vec<&rustway::Kp> = store
        .placement
        .keys()
        .filter_map(|id| store.kps.get(id))
        .collect();
    let with_prereqs = placed.iter().filter(|k| !k.prereqs.is_empty()).count();
    let ratio = with_prereqs as f64 / placed.len().max(1) as f64;
    assert!(
        ratio >= 0.5,
        "有前置依赖的知识点比例过低（{:.0}%），知识图谱将退化成散点",
        ratio * 100.0
    );
}

#[test]
fn api_manifest_is_consistent() {
    let store = real_store();
    let manifest = store.manifest_json();
    assert_eq!(manifest["stats"]["kps"], serde_json::json!(store.kps.len()));
    let modules = manifest["modules"].as_array().unwrap();
    assert_eq!(modules.len(), store.curriculum.modules.len());
    // 抽查一条边
    let edges = manifest["edges"].as_array().unwrap();
    assert!(!edges.is_empty(), "知识图谱没有边");
}

#[test]
fn kp_detail_api_returns_rendered_html() {
    let store = real_store();
    // 找一个带 detail_md 的知识点
    let with_detail = store
        .kps
        .values()
        .find(|k| k.detail_md.is_some())
        .expect("至少应有一个知识点带详细文档");
    let kp = store
        .kp_json(&with_detail.id)
        .unwrap_or_else(|| panic!("kp_json 返回 None: {}", with_detail.id));
    let html = kp["detailHtml"].as_str().expect("detailHtml");
    assert!(
        html.contains("<h2>") || html.contains("<h1>") || html.contains("<p>"),
        "渲染后的正文不含 HTML 结构"
    );
}
