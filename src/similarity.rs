//! 知识点相似度引擎。
//!
//! 基于加权词袋 + 余弦相似度，在知识点之间自动发现「相似关联」边，
//! 与手工声明的前置依赖（prereqs）互补，共同构成知识图谱的关联体系。
//!
//! 分词策略（面向中英混排）：
//! - ASCII 连续字母数字 → 小写词（len ≥ 2）
//! - CJK 连续段 → 二元组（bigram）；单字段保留单字
//!
//! 字段权重：tags ×4（人工标注的语义锚点最可信）、title ×3、summary ×1、outline ×1。
//! 确定性：纯函数、无随机，同一内容永远产生同一张相似图。

use crate::Kp;
use std::collections::{HashMap, HashSet};

/// 一条相似关联边（无向；a < b 保证唯一）。
#[derive(Debug, Clone, PartialEq)]
pub struct SimilarEdge {
    pub a: String,
    pub b: String,
    pub score: f32,
}

fn tokenize(text: &str, weight: f32, tokens: &mut HashMap<String, f32>) {
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut ascii_word = String::new();
    let mut cjk_run: Vec<char> = Vec::new();

    let flush_ascii = |w: &mut String, out: &mut HashMap<String, f32>| {
        if w.chars().count() >= 2 {
            *out.entry(w.clone()).or_insert(0.0) += weight;
        }
        w.clear();
    };
    let flush_cjk = |run: &mut Vec<char>, out: &mut HashMap<String, f32>| {
        if run.len() == 1 {
            *out.entry(run[0].to_string()).or_insert(0.0) += weight;
        } else {
            for w in run.windows(2) {
                *out.entry(w.iter().collect()).or_insert(0.0) += weight;
            }
        }
        run.clear();
    };

    for &c in &chars {
        if c.is_ascii_alphanumeric() {
            flush_cjk(&mut cjk_run, tokens);
            ascii_word.push(c);
        } else if is_cjk(c) {
            flush_ascii(&mut ascii_word, tokens);
            cjk_run.push(c);
        } else {
            flush_ascii(&mut ascii_word, tokens);
            flush_cjk(&mut cjk_run, tokens);
        }
    }
    flush_ascii(&mut ascii_word, tokens);
    flush_cjk(&mut cjk_run, tokens);
}

fn is_cjk(c: char) -> bool {
    // CJK 统一表意文字 + 扩展A + 兼容表意
    ('\u{4E00}'..='\u{9FFF}').contains(&c)
        || ('\u{3400}'..='\u{4DBF}').contains(&c)
        || ('\u{F900}'..='\u{FAFF}').contains(&c)
}

/// 计算一个知识点的加权词袋。
fn kp_vector(kp: &Kp) -> HashMap<String, f32> {
    let mut v = HashMap::new();
    tokenize(&kp.title, 3.0, &mut v);
    for t in &kp.tags {
        tokenize(t, 4.0, &mut v);
    }
    tokenize(&kp.summary, 1.0, &mut v);
    for o in &kp.outline {
        tokenize(o, 1.0, &mut v);
    }
    v
}

fn cosine(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    if a.len() > b.len() {
        return cosine(b, a);
    }
    let mut dot = 0.0;
    for (k, x) in a {
        if let Some(y) = b.get(k) {
            dot += x * y;
        }
    }
    if dot == 0.0 {
        return 0.0;
    }
    let norm = |m: &HashMap<String, f32>| m.values().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm(a) * norm(b))
}

/// 全库两两计算，每个知识点保留相似度最高且 ≥ threshold 的 top_k 个邻居。
/// 边去重：(a,b) 与 (b,a) 视为同一条，保留并集视角下的最高分。
pub fn build_similarity_edges(
    kps: &HashMap<String, Kp>,
    threshold: f32,
    top_k: usize,
) -> Vec<SimilarEdge> {
    const MIN_PAIRS: usize = 2;
    if kps.len() < MIN_PAIRS {
        return Vec::new();
    }
    let ids: Vec<&String> = kps.keys().collect();
    let vectors: HashMap<&String, HashMap<String, f32>> =
        ids.iter().map(|id| (*id, kp_vector(&kps[*id]))).collect();

    // 邻居表：id -> [(neighbor, score)]
    let mut best: HashMap<&String, Vec<(String, f32)>> = HashMap::new();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let (a, b) = (ids[i], ids[j]);
            let score = cosine(&vectors[a], &vectors[b]);
            if score < threshold {
                continue;
            }
            best.entry(a).or_default().push(((*b).clone(), score));
            best.entry(b).or_default().push(((*a).clone(), score));
        }
    }

    let mut edges: Vec<SimilarEdge> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for (id, mut neighbors) in best {
        neighbors.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
        for (other, score) in neighbors.into_iter().take(top_k) {
            let key = if *id < other {
                (id.clone(), other)
            } else {
                (other, id.clone())
            };
            if seen.insert(key.clone()) {
                edges.push(SimilarEdge {
                    a: key.0,
                    b: key.1,
                    score,
                });
            }
        }
    }
    edges.sort_by(|x, y| {
        y.score
            .partial_cmp(&x.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    edges
}

/// 某个知识点的相似邻居列表（按分数降序）。
pub fn related_of(edges: &[SimilarEdge], id: &str) -> Vec<(String, f32)> {
    let mut out: Vec<(String, f32)> = edges
        .iter()
        .filter(|e| e.a == id || e.b == id)
        .map(|e| {
            if e.a == id {
                (e.b.clone(), e.score)
            } else {
                (e.a.clone(), e.score)
            }
        })
        .collect();
    out.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Kp;

    fn kp(id: &str, title: &str, tags: &[&str], summary: &str) -> Kp {
        Kp {
            id: id.into(),
            title: title.into(),
            difficulty: 1,
            minutes: 10,
            summary: summary.into(),
            outline: vec![],
            prereqs: vec![],
            task: "任务".into(),
            refs: vec![],
            quiz: vec![],
            detail_md: None,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            detail_html: String::new(),
        }
    }

    #[test]
    fn similar_kps_get_an_edge() {
        let mut m = HashMap::new();
        m.insert(
            "a".into(),
            kp(
                "a",
                "所有权三规则",
                &["所有权", "核心"],
                "每个值只有一个所有者，离开作用域即 drop",
            ),
        );
        m.insert(
            "b".into(),
            kp(
                "b",
                "Move 与 Copy 语义",
                &["所有权", "核心"],
                "赋值默认 move 转移所有权，Copy 类型按位复制",
            ),
        );
        m.insert(
            "c".into(),
            kp(
                "c",
                "HTTP 状态码",
                &["网络", "HTTP"],
                "2xx 成功 3xx 重定向 4xx 客户端错误",
            ),
        );
        let edges = build_similarity_edges(&m, 0.15, 3);
        let has_ab = edges
            .iter()
            .any(|e| (e.a == "a" && e.b == "b") || (e.a == "b" && e.a != e.b) && (e.b == "a"));
        assert!(has_ab, "所有权两节点应有相似边，实际: {edges:?}");
        let has_ac = edges
            .iter()
            .any(|e| (e.a == "a" && e.b == "c") || (e.a == "c" && e.b == "a"));
        assert!(!has_ac, "所有权与 HTTP 不应相关");
    }

    #[test]
    fn edges_are_deterministic_and_symmetric() {
        let build = || {
            let mut m = HashMap::new();
            m.insert(
                "x".into(),
                kp("x", "二分查找", &["算法"], "有序空间每次减半的查找"),
            );
            m.insert(
                "y".into(),
                kp("y", "二分答案", &["算法"], "对答案空间二分求解"),
            );
            build_similarity_edges(&m, 0.15, 3)
        };
        let e1 = build();
        let e2 = build();
        assert_eq!(e1, e2, "同一内容两次计算结果必须一致");
        assert!(e1.iter().all(|e| e.a < e.b), "边端点应有序去重");
    }

    #[test]
    fn related_of_returns_sorted_neighbors() {
        let mut m = HashMap::new();
        m.insert(
            "a".into(),
            kp("a", "哈希表", &["数据结构"], "哈希函数与冲突解决"),
        );
        m.insert(
            "b".into(),
            kp("b", "哈希函数", &["数据结构", "哈希"], "均匀映射到桶"),
        );
        m.insert(
            "c".into(),
            kp("c", "布隆过滤器", &["哈希"], "概率型成员查询"),
        );
        let edges = build_similarity_edges(&m, 0.10, 3);
        let rel = related_of(&edges, "a");
        assert!(!rel.is_empty());
        assert!(rel.windows(2).all(|w| w[0].1 >= w[1].1), "应按分数降序");
    }

    #[test]
    fn single_kp_has_no_edges() {
        let mut m = HashMap::new();
        m.insert(
            "solo".into(),
            kp("solo", "孤点", &["测试"], "只有一个知识点"),
        );
        assert!(build_similarity_edges(&m, 0.15, 3).is_empty());
    }
}
