//! rustway — 内容驱动的计算机学习教程引擎。
//!
//! 数据流：`content/curriculum.json`（章节结构）+ `content/kps/*.json`（知识点，任意文件自动合并）
//! → 加载与校验 → axum API → 前端知识图谱。
//! 新增知识点**不需要修改任何 Rust 代码**：新建 JSON 文件、在 curriculum.json 中挂载即可。

pub mod api;
pub mod db;
pub mod similarity;

use serde::{Deserialize, Serialize};

/// 知识点加载规范（JSON Schema 2020-12），随二进制发布：
/// 社区按规范编写 JSON 即可通过 `rustway import <file>` 校验入库。
pub const KP_SCHEMA: &str = include_str!("../schema/kp.schema.json");
pub const KP_FILE_SCHEMA: &str = include_str!("../schema/kp-file.schema.json");
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------- 数据模型

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Curriculum {
    pub title: String,
    pub subtitle: String,
    pub version: String,
    pub methodology: Methodology,
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Methodology {
    pub intro: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Module {
    pub id: String,
    pub num: u32,
    pub title: String,
    pub icon: String,
    pub color: String,
    pub subtitle: String,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub objectives: Vec<String>,
    pub kps: Vec<String>,
    pub checkpoint: Vec<QuizItem>,
}

/// 一道测验题。KP 自带测验的 `kp` 字段由所属知识点推导，关卡测验必须显式标注。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuizItem {
    pub q: String,
    pub opts: Vec<String>,
    pub answer: usize,
    pub why: String,
    #[serde(default)]
    pub kp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Kp {
    pub id: String,
    pub title: String,
    pub difficulty: u32,
    pub minutes: u32,
    pub summary: String,
    #[serde(default)]
    pub outline: Vec<String>,
    #[serde(default)]
    pub prereqs: Vec<String>,
    pub task: String,
    #[serde(default)]
    pub refs: Vec<RefLink>,
    #[serde(default)]
    pub quiz: Vec<QuizItem>,
    #[serde(default)]
    pub detail_md: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 加载时渲染好的正文 HTML（detail_md 存在时）
    #[serde(skip)]
    pub detail_html: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefLink {
    pub t: String,
    pub u: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KpFile {
    kps: Vec<Kp>,
}

// ---------------------------------------------------------------- Store

#[derive(Debug)]
pub struct Store {
    pub curriculum: Curriculum,
    /// 全部知识点（含未编排），按 id 索引
    pub kps: HashMap<String, Kp>,
    /// kp id → chapter id（仅已编排者）
    pub placement: HashMap<String, String>,
    /// 已编排但未挂入任何章节的知识点 id（「待编排」）
    pub unplaced: Vec<String>,
    /// 校验错误（阻塞项：cargo test / CI 会失败）
    pub errors: Vec<String>,
    /// 警告（非阻塞：建议修复的质量问题，API 与 validate 命令可见）
    pub warnings: Vec<String>,
    /// 相似算法自动发现的关联边（与手工前置互补）
    pub similar_edges: Vec<similarity::SimilarEdge>,
}

impl Store {
    pub fn module_of_kp(&self, kp_id: &str) -> Option<&Module> {
        self.curriculum.modules.iter().find(|m| {
            m.chapters
                .iter()
                .any(|c| self.placement.get(kp_id) == Some(&c.id))
        })
    }

    pub fn total_minutes(&self) -> u32 {
        self.kps.values().map(|k| k.minutes).sum()
    }

    pub fn total_quiz(&self) -> usize {
        self.kps.values().map(|k| k.quiz.len()).sum::<usize>()
            + self
                .curriculum
                .modules
                .iter()
                .flat_map(|m| m.chapters.iter())
                .map(|c| c.checkpoint.len())
                .sum::<usize>()
    }
}

// ---------------------------------------------------------------- 加载

/// 读取并合并内容目录，返回带校验结果的 Store。
/// 任何文件级错误（JSON 语法、缺失清单）直接报错；内容级问题收集进 `errors`。
pub fn load_store(content_dir: &Path) -> Result<Store, String> {
    let manifest_path = content_dir.join("curriculum.json");
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("读取 {} 失败: {e}", manifest_path.display()))?;
    let curriculum: Curriculum =
        serde_json::from_str(&raw).map_err(|e| format!("解析 curriculum.json 失败: {e}"))?;

    // 仓库根目录（detail_md 路径以仓库根为基准，如 "content/rust/ownership.md"）
    let repo_root = content_dir.parent().unwrap_or(Path::new("."));

    // 合并 content/kps/*.json —— 任意新增文件自动被发现（「任意插入」的基石）
    // 以下划线开头的文件（如 _draft.json、_TEMPLATE.json）视为草稿，不参与发布
    let kps_dir = content_dir.join("kps");
    let mut kps: HashMap<String, Kp> = HashMap::new();
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut entries: Vec<_> = match fs::read_dir(&kps_dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(e) => return Err(format!("读取 {} 失败: {e}", kps_dir.display())),
    };
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.starts_with('_'))
            .unwrap_or(false)
        {
            continue; // 草稿/模板：跳过
        }
        let text =
            fs::read_to_string(&path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
        let file: KpFile = serde_json::from_str(&text)
            .map_err(|e| format!("解析 {} 失败: {e}", path.display()))?;
        for mut kp in file.kps {
            // 预渲染正文
            kp.detail_html = kp
                .detail_md
                .as_deref()
                .and_then(|rel| fs::read_to_string(repo_root.join(rel)).ok())
                .map(|md| md_to_html(&md))
                .unwrap_or_default();
            if let Some(old) = kps.insert(kp.id.clone(), kp) {
                errors.push(format!(
                    "知识点 id 重复: {}（旧定义来自先前文件，冲突文件 {}）",
                    old.id,
                    path.display()
                ));
            }
        }
    }

    // ---- 校验与编排索引 ----
    let mut placement: HashMap<String, String> = HashMap::new();
    let mut unplaced: Vec<String> = kps.keys().cloned().collect();
    unplaced.sort();

    // 章节引用
    let mut chapter_ids: HashSet<&str> = HashSet::new();
    let mut module_nums: HashSet<u32> = HashSet::new();
    for module in &curriculum.modules {
        if !chapter_ids.insert(&module.id) {
            errors.push(format!("模块 id 重复: {}", module.id));
        }
        if !module_nums.insert(module.num) {
            errors.push(format!("模块编号 num 重复: {}", module.num));
        }
        for chapter in &module.chapters {
            if !chapter_ids.insert(&chapter.id) {
                errors.push(format!("章节 id 重复: {}", chapter.id));
            }
            if chapter.objectives.is_empty() {
                warnings.push(format!("章节 {} 缺少学习目标（objectives）", chapter.id));
            }
            for kp_id in &chapter.kps {
                if !kps.contains_key(kp_id) {
                    errors.push(format!(
                        "章节 {} 引用了不存在的知识点: {}",
                        chapter.id, kp_id
                    ));
                    continue;
                }
                unplaced.retain(|id| id != kp_id);
                if placement
                    .insert(kp_id.clone(), chapter.id.clone())
                    .is_some()
                {
                    errors.push(format!("知识点 {} 被编排到多个章节", kp_id));
                }
            }
            for q in &chapter.checkpoint {
                if q.kp.is_empty() {
                    errors.push(format!("章节 {} 的关卡测验题缺少 kp 字段", chapter.id));
                } else if !chapter.kps.contains(&q.kp) {
                    errors.push(format!(
                        "章节 {} 的关卡测验题指向了本章之外的知识点: {}",
                        chapter.id, q.kp
                    ));
                }
                validate_quiz(&q.q, &q.opts, q.answer, &q.why, &chapter.id, &mut errors);
            }
        }
    }

    // 知识点字段
    for (id, kp) in &kps {
        if !(1..=3).contains(&kp.difficulty) {
            errors.push(format!(
                "知识点 {id} 的 difficulty 必须是 1/2/3（当前 {}）",
                kp.difficulty
            ));
        }
        if kp.minutes == 0 || kp.minutes > 600 {
            warnings.push(format!(
                "知识点 {id} 的 minutes 超出合理范围（1..=600）：{}",
                kp.minutes
            ));
        }
        if kp.summary.chars().count() > 80 {
            warnings.push(format!("知识点 {id} 的摘要超过 80 字，图谱提示会显示不全"));
        }
        if kp.outline.is_empty() && kp.detail_md.is_none() {
            warnings.push(format!(
                "知识点 {id} 既无详细文档也无大纲（outline），页面上将没有正文"
            ));
        }
        if !kp
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            warnings.push(format!(
                "知识点 {id} 的 id 不符合建议格式（小写字母/数字/连字符）"
            ));
        }
        if kp.refs.is_empty() {
            errors.push(format!("知识点 {id} 缺少引用出处（refs 为空）"));
        }
        let mut seen_urls: HashSet<&str> = HashSet::new();
        for r in &kp.refs {
            if !(r.u.starts_with("https://") || r.u.starts_with("http://")) {
                errors.push(format!("知识点 {id} 的引用不是 http(s) 链接: {}", r.u));
            }
            if r.t.is_empty() {
                errors.push(format!("知识点 {id} 有引用缺少标题: {}", r.u));
            }
            if !seen_urls.insert(r.u.as_str()) {
                warnings.push(format!("知识点 {id} 存在重复引用: {}", r.u));
            }
        }
        if kp.quiz.is_empty() {
            errors.push(format!("知识点 {id} 没有任何自测题（quiz 为空）"));
        }
        for q in &kp.quiz {
            validate_quiz(&q.q, &q.opts, q.answer, &q.why, id, &mut errors);
        }
        if kp.task.trim().is_empty() {
            errors.push(format!("知识点 {id} 缺少验证任务（task 为空）"));
        }
        if kp.summary.trim().is_empty() {
            errors.push(format!("知识点 {id} 缺少摘要"));
        }
        if let Some(rel) = &kp.detail_md {
            let p = repo_root.join(rel);
            if !p.exists() {
                errors.push(format!("知识点 {id} 的 detail_md 文件不存在: {rel}"));
            } else if fs::metadata(&p).map(|m| m.len() == 0).unwrap_or(false) {
                warnings.push(format!("知识点 {id} 的 detail_md 是空文件: {rel}"));
            }
        }
        for p in &kp.prereqs {
            if !kps.contains_key(p) {
                errors.push(format!("知识点 {id} 的前置 {} 不存在", p));
            }
        }
    }

    // 前置依赖图不能有环（Kahn 拓扑排序：无法处理完所有节点 = 存在环）
    if let Some(detail) = detect_cycle(&kps) {
        errors.push(format!("知识点前置依赖存在环: {detail}"));
    }

    // 相似算法自动关联：新插入的知识点即使未声明任何前置，
    // 也会按语义相似度接入图中最相关的节点（「任意插入」的另一半保障）
    let similar_edges = similarity::build_similarity_edges(&kps, 0.16, 3);

    // 图谱孤岛检测（无前置也无后继的节点会让图谱退化成散点）
    let mut has_dependent: HashSet<&str> = HashSet::new();
    for kp in kps.values() {
        for p in &kp.prereqs {
            if kps.contains_key(p.as_str()) {
                has_dependent.insert(p.as_str());
            }
        }
    }
    for (id, kp) in &kps {
        if kp.prereqs.is_empty() && !has_dependent.contains(id.as_str()) {
            warnings.push(format!(
                "知识点 {id} 是知识图谱孤岛（无前置也无后继），建议接入依赖链"
            ));
        }
    }

    Ok(Store {
        curriculum,
        kps,
        placement,
        unplaced,
        errors,
        warnings,
        similar_edges,
    })
}

fn validate_quiz(
    q: &str,
    opts: &[String],
    answer: usize,
    why: &str,
    owner: &str,
    errors: &mut Vec<String>,
) {
    if q.trim().is_empty() {
        errors.push(format!("{owner} 存在空题干"));
    }
    if opts.len() < 2 {
        errors.push(format!("{owner} 的题目选项不足 2 个: {q}"));
    }
    if answer >= opts.len() {
        errors.push(format!("{owner} 的题目答案下标越界: {q}"));
    }
    if why.trim().is_empty() {
        errors.push(format!("{owner} 的题目缺少解析: {q}"));
    }
}

/// 用 Kahn 拓扑排序检测前置图中是否有环；有环返回描述。
fn detect_cycle(kps: &HashMap<String, Kp>) -> Option<String> {
    // 入度 = 该知识点尚未满足的现存前置数；边方向：前置 -> 知识点
    let mut indeg: HashMap<&str, usize> = kps.keys().map(|k| (k.as_str(), 0)).collect();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    for (id, kp) in kps {
        for p in &kp.prereqs {
            if kps.contains_key(p) {
                *indeg.get_mut(id.as_str()).unwrap() += 1;
                dependents.entry(p.as_str()).or_default().push(id.as_str());
            }
        }
    }
    let mut queue: Vec<&str> = indeg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(k, _)| *k)
        .collect();
    let mut processed = 0usize;
    while let Some(n) = queue.pop() {
        processed += 1;
        if let Some(deps) = dependents.get(n) {
            for &d in deps {
                let e = indeg.get_mut(d).unwrap();
                *e -= 1;
                if *e == 0 {
                    queue.push(d);
                }
            }
        }
    }
    if processed < kps.len() {
        let stuck: Vec<String> = kps
            .iter()
            .filter(|(id, kp)| {
                indeg.get(id.as_str()).copied().unwrap_or(0) > 0
                    && kp.prereqs.iter().all(|p| kps.contains_key(p))
            })
            .map(|(id, _)| id.clone())
            .collect();
        Some(format!(
            "{} 个知识点无法排入学习顺序，疑似环成员: {}",
            kps.len() - processed,
            stuck.join(", ")
        ))
    } else {
        None
    }
}

// ---------------------------------------------------------------- Markdown 渲染

pub fn md_to_html(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, opts);
    let mut out = String::with_capacity(md.len() * 2);
    html::push_html(&mut out, parser);
    out
}

// ---------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_kp(id: &str) -> Kp {
        Kp {
            id: id.into(),
            title: format!("标题 {id}"),
            difficulty: 1,
            minutes: 10,
            summary: "摘要".into(),
            outline: vec!["要点一".into()],
            prereqs: vec![],
            task: "完成任务 X".into(),
            refs: vec![RefLink {
                t: "官方文档".into(),
                u: "https://doc.rust-lang.org/".into(),
            }],
            quiz: vec![QuizItem {
                q: "问题？".into(),
                opts: vec!["对".into(), "错".into()],
                answer: 0,
                why: "因为……".into(),
                kp: String::new(),
            }],
            detail_md: None,
            tags: vec![],
            detail_html: String::new(),
        }
    }

    fn write_tmp_content(
        tag: &str,
        curriculum: &str,
        kp_files: &[(&str, &str)],
    ) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustway-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("kps")).unwrap();
        fs::write(dir.join("curriculum.json"), curriculum).unwrap();
        for (name, body) in kp_files {
            fs::write(dir.join("kps").join(format!("{name}.json")), body).unwrap();
        }
        dir
    }

    /// 最小合法清单：章节 c1 只编排 a
    const MIN_CURRICULUM: &str = r##"{
        "title": "t", "subtitle": "s", "version": "0",
        "methodology": {"intro": "i", "rules": []},
        "modules": [{"id": "m1", "num": 0, "title": "M", "icon": "x", "color": "#000", "subtitle": "s",
            "chapters": [{"id": "c1", "title": "C", "objectives": [], "kps": ["a"], "checkpoint": []}]}]
    }"##;
    /// 引用了不存在知识点 b 的清单
    const BAD_REF_CURRICULUM: &str = r##"{
        "title": "t", "subtitle": "s", "version": "0",
        "methodology": {"intro": "i", "rules": []},
        "modules": [{"id": "m1", "num": 0, "title": "M", "icon": "x", "color": "#000", "subtitle": "s",
            "chapters": [{"id": "c1", "title": "C", "objectives": [], "kps": ["a", "b"], "checkpoint": []}]}]
    }"##;

    fn kp_file(body: &str) -> String {
        format!("{{\"kps\": [{body}]}}")
    }

    #[test]
    fn missing_kp_reference_is_reported() {
        let dir = write_tmp_content(
            "ok",
            BAD_REF_CURRICULUM,
            &[(
                "a",
                &kp_file(&serde_json::to_string(&fixture_kp("a")).unwrap()),
            )],
        );
        // b 未提供文件 → 章节引用报错
        let store = load_store(&dir).unwrap();
        assert!(!store.errors.is_empty(), "缺少 kp b 应报错");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn orphan_kps_are_unplaced_not_errors() {
        let mut a = fixture_kp("a");
        a.id = "b".into();
        let dir = write_tmp_content(
            "orphan",
            MIN_CURRICULUM,
            &[
                (
                    "a",
                    &kp_file(&serde_json::to_string(&fixture_kp("a")).unwrap()),
                ),
                ("b", &kp_file(&serde_json::to_string(&a).unwrap())),
            ],
        );
        let store = load_store(&dir).unwrap();
        assert!(store.errors.is_empty(), "errors: {:?}", store.errors);
        assert_eq!(store.placement.len(), 1);
        assert_eq!(store.unplaced, vec!["b".to_string()]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_prereq_is_reported() {
        let mut a = fixture_kp("a");
        a.prereqs = vec!["ghost".into()];
        let dir = write_tmp_content(
            "prereq",
            MIN_CURRICULUM,
            &[("a", &kp_file(&serde_json::to_string(&a).unwrap()))],
        );
        let store = load_store(&dir).unwrap();
        assert!(store.errors.iter().any(|e| e.contains("ghost")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn prereq_cycle_is_detected() {
        let mut a = fixture_kp("a");
        let mut b = fixture_kp("b");
        a.prereqs = vec!["b".into()];
        b.prereqs = vec!["a".into()];
        let dir = write_tmp_content(
            "cycle",
            MIN_CURRICULUM,
            &[
                ("a", &kp_file(&serde_json::to_string(&a).unwrap())),
                ("b", &kp_file(&serde_json::to_string(&b).unwrap())),
            ],
        );
        let store = load_store(&dir).unwrap();
        assert!(store.errors.iter().any(|e| e.contains("环")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn http_reference_is_rejected() {
        let mut a = fixture_kp("a");
        a.refs[0].u = "ftp://example.com/doc".into();
        let dir = write_tmp_content(
            "ref",
            MIN_CURRICULUM,
            &[("a", &kp_file(&serde_json::to_string(&a).unwrap()))],
        );
        let store = load_store(&dir).unwrap();
        assert!(store.errors.iter().any(|e| e.contains("http(s)")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn markdown_renders_to_html() {
        let html = md_to_html("# 标题\n\n- 甲\n- 乙");
        assert!(html.contains("<h1>标题</h1>"));
        assert!(html.contains("<li>甲</li>"));
    }

    #[test]
    fn cycle_helper_reports_directly() {
        let mut a = fixture_kp("a");
        let mut b = fixture_kp("b");
        a.prereqs = vec!["b".into()];
        b.prereqs = vec!["a".into()];
        let mut map = HashMap::new();
        map.insert("a".into(), a);
        map.insert("b".into(), b);
        assert!(detect_cycle(&map).is_some());
    }

    #[test]
    fn draft_files_prefixed_with_underscore_are_skipped() {
        let kp = serde_json::to_string(&fixture_kp("a")).unwrap();
        let dir = write_tmp_content("draft", MIN_CURRICULUM, &[("a", &kp_file(&kp))]);
        // 草稿文件里的 id 与正式内容冲突 → 若未跳过会报重复错误
        fs::write(
            dir.join("kps").join("_draft.json"),
            kp_file(&kp.replace("\"a\"", "\"b\"")),
        )
        .unwrap();
        let store = load_store(&dir).unwrap();
        assert!(store.errors.is_empty(), "errors: {:?}", store.errors);
        assert_eq!(store.kps.len(), 1, "草稿中的 b 不应被加载");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_difficulty_is_an_error() {
        let mut a = fixture_kp("a");
        a.difficulty = 5;
        let dir = write_tmp_content(
            "diff",
            MIN_CURRICULUM,
            &[("a", &kp_file(&serde_json::to_string(&a).unwrap()))],
        );
        let store = load_store(&dir).unwrap();
        assert!(store
            .errors
            .iter()
            .any(|e| e.contains("difficulty 必须是 1/2/3")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn isolated_kp_produces_warning_not_error() {
        // 孤岛知识点（无前置无后继）应产生警告而非错误
        let dir = write_tmp_content(
            "island",
            MIN_CURRICULUM,
            &[(
                "a",
                &kp_file(&serde_json::to_string(&fixture_kp("a")).unwrap()),
            )],
        );
        let store = load_store(&dir).unwrap();
        assert!(store.errors.is_empty());
        assert!(
            store.warnings.iter().any(|w| w.contains("知识图谱孤岛")),
            "warnings: {:?}",
            store.warnings
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_http_reference_is_rejected() {
        let mut a = fixture_kp("a");
        a.refs[0].u = "ftp://docs.example.com/x".into();
        let dir = write_tmp_content(
            "ftp",
            MIN_CURRICULUM,
            &[("a", &kp_file(&serde_json::to_string(&a).unwrap()))],
        );
        let store = load_store(&dir).unwrap();
        assert!(store.errors.iter().any(|e| e.contains("http(s)")));
        fs::remove_dir_all(&dir).ok();
    }
}
