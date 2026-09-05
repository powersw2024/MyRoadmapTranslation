//! 内容完整性集成测试 —— 本框架「可验证」承诺的强制执行者。
//!
//! 这些测试与内容体量无关（内容库可从零开始重建）：
//! 1. 任何已加载的内容都必须通过全部完整性规则（引用存在、前置无环、出处合法…）
//! 2. 已编排的章节必须非空且带关卡测验
//! 3. manifest / 相似边 / related 接口的自洽性
//!
//! 体量类断言（如「知识点不少于 N 个」）仅在内容规模足以支撑统计时才生效，
//! 避免在内容重建期阻塞提交。

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
fn knowledge_graph_connectivity_when_nontrivial() {
    // 图谱的价值在于前置关系：内容规模足够时，多数知识点应有前置
    let store = real_store();
    if store.placement.len() < 10 {
        return; // 内容重建期：图太小，比例无统计意义
    }
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
    if store.kps.len() >= 2 {
        let edges = manifest["edges"].as_array().unwrap();
        assert!(!edges.is_empty(), "多于一个知识点时图谱必须有边");
    }
}

#[test]
fn similarity_engine_edges_are_valid() {
    let store = real_store();
    // 大体量内容下应自动发现可观的关联边；小体量时仅校验已有边的合法性
    if store.kps.len() >= 50 {
        assert!(
            store.similar_edges.len() >= 30,
            "相似关联边过少（{}），算法阈值或分词可能有问题",
            store.similar_edges.len()
        );
    }
    for e in &store.similar_edges {
        assert!(e.score > 0.0 && e.score <= 1.0, "非法分数: {:?}", e);
        assert!(e.a < e.b, "边端点应有序: {:?}", e);
        assert!(store.kps.contains_key(&e.a) && store.kps.contains_key(&e.b));
    }
    let manifest = store.manifest_json();
    let similar = manifest["similarEdges"].as_array().unwrap();
    assert_eq!(similar.len(), store.similar_edges.len());
}

#[test]
fn kp_api_exposes_algorithmic_related_list() {
    let store = real_store();
    let Some(e) = store.similar_edges.first() else {
        return; // 无相似边（内容过少）时跳过
    };
    let kp = store.kp_json(&e.a).expect("kp_json");
    let related = kp["related"].as_array().expect("related").clone();
    assert!(
        !related.is_empty(),
        "知识点 {} 应有算法推荐的关联知识点",
        e.a
    );
    let first = &related[0];
    assert!(first["score"].as_u64().unwrap() >= 10, "分数应为百分制");
}

#[test]
fn kp_detail_api_returns_rendered_html() {
    let store = real_store();
    // 找一个带 detail_md 的知识点；重建期允许暂时没有详细文档
    let Some(with_detail) = store.kps.values().find(|k| k.detail_md.is_some()) else {
        return;
    };
    let kp = store
        .kp_json(&with_detail.id)
        .unwrap_or_else(|| panic!("kp_json 返回 None: {}", with_detail.id));
    let html = kp["detailHtml"].as_str().expect("detailHtml");
    assert!(
        html.contains("<h2>") || html.contains("<h1>") || html.contains("<p>"),
        "渲染后的正文不含 HTML 结构"
    );
}
