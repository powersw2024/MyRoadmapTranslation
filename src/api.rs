//! API 响应构建：把 Store 序列化为前端需要的 JSON。

use crate::Store;
use serde_json::{json, Value};

impl Store {
    /// 首页知识图谱与课程树所需的完整清单。
    pub fn manifest_json(&self) -> Value {
        let mut modules = Vec::new();
        for m in &self.curriculum.modules {
            let mut chapters = Vec::new();
            for c in &m.chapters {
                let kps: Vec<Value> = c
                    .kps
                    .iter()
                    .filter_map(|id| self.kps.get(id))
                    .map(|k| {
                        json!({
                            "id": k.id, "title": k.title,
                            "difficulty": k.difficulty, "minutes": k.minutes,
                            "summary": k.summary, "prereqs": k.prereqs,
                            "tags": k.tags,
                            "moduleId": m.id, "chapterId": c.id,
                            "hasDetail": !k.detail_html.is_empty(),
                        })
                    })
                    .collect();
                chapters.push(json!({
                    "id": c.id, "title": c.title,
                    "objectives": c.objectives, "kps": kps,
                    "checkpoint": c.checkpoint, "moduleId": m.id,
                }));
            }
            modules.push(json!({
                "id": m.id, "num": m.num, "title": m.title,
                "icon": m.icon, "color": m.color, "subtitle": m.subtitle,
                "chapters": chapters,
            }));
        }

        // 前置依赖边：[前置, 知识点]
        let edges: Vec<Value> = self
            .kps
            .values()
            .flat_map(|k| k.prereqs.iter().map(move |p| (p, &k.id)))
            .filter(|(p, k)| self.kps.contains_key(*p) && self.kps.contains_key(*k))
            .map(|(p, k)| json!([p, k]))
            .collect();

        // 相似算法边：[a, b, 分数]
        let similar_edges: Vec<Value> = self
            .similar_edges
            .iter()
            .map(|e| json!([e.a, e.b, (e.score * 1000.0).round() / 1000.0]))
            .collect();

        let unplaced: Vec<Value> = self
            .unplaced
            .iter()
            .filter_map(|id| self.kps.get(id))
            .map(|k| {
                json!({
                    "id": k.id, "title": k.title, "difficulty": k.difficulty,
                    "minutes": k.minutes, "summary": k.summary, "tags": k.tags,
                })
            })
            .collect();

        json!({
            "title": self.curriculum.title,
            "subtitle": self.curriculum.subtitle,
            "version": self.curriculum.version,
            "methodology": self.curriculum.methodology,
            "stats": {
                "modules": self.curriculum.modules.len(),
                "chapters": self.curriculum.modules.iter().map(|m| m.chapters.len()).sum::<usize>(),
                "kps": self.kps.len(),
                "placed": self.placement.len(),
                "unplaced": self.unplaced.len(),
                "minutes": self.total_minutes(),
                "quiz": self.total_quiz(),
                "warnings": self.warnings.len(),
            },
            "modules": modules,
            "unplaced": unplaced,
            "edges": edges,
            "similarEdges": similar_edges,
        })
    }

    /// 单个知识点的完整详情（含渲染后的正文与测验）。
    pub fn kp_json(&self, id: &str) -> Option<Value> {
        let k = self.kps.get(id)?;
        let module = self.module_of_kp(id);
        // 相似算法推荐的关联知识点（排除显式前置，附分数）
        let related: Vec<Value> = crate::similarity::related_of(&self.similar_edges, id)
            .into_iter()
            .filter(|(rid, _)| !k.prereqs.contains(rid))
            .filter_map(|(rid, score)| {
                self.kps.get(&rid).map(|r| {
                    json!({
                        "id": r.id, "title": r.title,
                        "score": (score * 100.0).round() as u32,
                    })
                })
            })
            .collect();
        json!({
            "id": k.id, "title": k.title, "difficulty": k.difficulty,
            "minutes": k.minutes, "summary": k.summary, "outline": k.outline,
            "prereqs": k.prereqs, "task": k.task,
            "refs": k.refs, "quiz": k.quiz, "tags": k.tags,
            "detailHtml": k.detail_html,
            "related": related,
            "moduleId": module.map(|m| m.id.clone()),
            "moduleTitle": module.map(|m| m.title.clone()),
            "moduleColor": module.map(|m| m.color.clone()),
            "chapterId": self.placement.get(id),
        })
        .pipe_some()
    }
}

trait PipeSome {
    fn pipe_some(self) -> Option<Value>;
}
impl PipeSome for Value {
    fn pipe_some(self) -> Option<Value> {
        Some(self)
    }
}
