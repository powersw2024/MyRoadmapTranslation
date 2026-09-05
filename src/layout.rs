//! 星系团布局引擎（服务端计算，前端零布局成本）。
//!
//! 视觉模型：
//! - 一个「知识面」（模块）= 一个星系扇区；
//! - 一颗「星球」（知识点）按**核心度**分层入轨：度数越高（被依赖/依赖/相似关联越多）
//!   轨道半径越小，越靠近星系中心；
//! - 同轨道上，同一知识面的星球聚集在该模块的扇区角度带内运行；
//! - 完全孤立的节点放最外圈（外围尘埃带）。
//!
//! 确定性：排序全部以 (score desc, id asc) 决胜负，同一内容产出同一布局。
//! 前端只消费 (x, y, orbit, core)，不做任何力导向计算。

use crate::Store;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct GalaxyNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    /// 轨道编号（0 = 最内圈）
    pub orbit: u32,
    /// 核心度（边度数）
    pub core: u32,
}

pub struct Galaxy {
    pub nodes: Vec<GalaxyNode>,
}

/// 计算星系布局。
pub fn build_galaxy(store: &Store) -> Galaxy {
    // 1) 核心度 = 前置边度数 + 相似边度数
    let mut core: HashMap<String, u32> = HashMap::new();
    let mut bump = |id: &str, by: u32| *core.entry(id.to_string()).or_insert(0) += by;
    for kp in store.kps.values() {
        for p in &kp.prereqs {
            if store.kps.contains_key(p.as_str()) {
                bump(p, 1);
                bump(&kp.id, 1);
            }
        }
    }
    for e in &store.similar_edges {
        bump(&e.a, 1);
        bump(&e.b, 1);
    }

    // 2) 排序定轨：核心度降序（同级按 id 稳定），孤岛排最外
    let mut ordered: Vec<(&String, u32)> = store
        .kps
        .keys()
        .map(|id| (id, core.get(id.as_str()).copied().unwrap_or(0)))
        .collect();
    ordered.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let isolated = ordered.iter().filter(|(_, c)| *c == 0).count();

    // 轨道容量：核心圈密、外圈疏（外圈周长更长）
    // 简化：等容量环；孤岛强制最外环
    let n_core = ordered.len() - isolated;
    let per_ring = ((n_core as f64).sqrt().ceil() as usize).clamp(6, 24);
    let rings = n_core.div_ceil(per_ring);
    let r_min = 150.0;
    let r_step = 68.0;
    let r_outer = r_min + rings as f64 * r_step;

    // 3) 角度：模块扇区 + 环内均匀展开
    let mods = &store.curriculum.modules;
    let n_mods = mods.len().max(1);
    let sector = std::f64::consts::TAU / n_mods as f64;
    let mod_index: HashMap<&str, usize> = mods
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.as_str(), i))
        .collect();

    // 分桶：(module, orbit) -> 计数器，用于环内扇区均匀摆位
    let mut bucket_seen: HashMap<(usize, u32), usize> = HashMap::new();

    let mut nodes = Vec::with_capacity(ordered.len());
    for (idx, (id, score)) in ordered.iter().enumerate() {
        let module_id = store
            .placement
            .get(*id)
            .and_then(|ch| {
                mods.iter()
                    .find(|m| m.chapters.iter().any(|c| &c.id == ch))
                    .map(|m| m.id.as_str())
            })
            .unwrap_or("");
        let mi = mod_index.get(module_id).copied().unwrap_or(0);

        let (orbit, radius) = if *score == 0 {
            (rings as u32 + 1, r_outer + r_step * 0.6)
        } else {
            let o = (idx / per_ring) as u32;
            (o, r_min + o as f64 * r_step)
        };

        // 环内扇区摆位：该 (module, orbit) 桶内第 k 个 → 扇区内均匀角度
        let k = bucket_seen.entry((mi, orbit)).or_insert(0);
        let in_bucket = *k;
        *k += 1;
        let base = mi as f64 * sector - std::f64::consts::FRAC_PI_2;
        // 扇区内留 8% 边距；孤岛环不按扇区（整圈铺开）
        let usable = if *score == 0 {
            std::f64::consts::TAU
        } else {
            sector * 0.92
        };
        let total_in_bucket = (in_bucket + 1).max(1) as f64;
        // 用桶内序号均匀取角：k/(cap) 难预知，用黄金比例散布保证均匀且确定
        let golden = 0.618_033_988_7_f64;
        let theta = if *score == 0 {
            base + (in_bucket as f64 + 0.5) * (std::f64::consts::TAU / 24.0) % std::f64::consts::TAU
        } else {
            base + sector * 0.04
                + usable * ((in_bucket as f64 + 0.5) / total_in_bucket)
                + (in_bucket as f64 * golden) % (usable / total_in_bucket)
        };

        nodes.push(GalaxyNode {
            id: (*id).clone(),
            x: 800.0 + radius * theta.cos(),
            y: 470.0 + radius * theta.sin(),
            orbit,
            core: *score,
        });
    }
    Galaxy { nodes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Kp;

    fn kp(id: &str) -> Kp {
        Kp {
            id: id.into(),
            title: format!("T{id}"),
            difficulty: 1,
            minutes: 10,
            summary: "s".into(),
            outline: vec![],
            prereqs: vec![],
            task: "t".into(),
            refs: vec![],
            quiz: vec![],
            detail_md: None,
            tags: vec![],
            domain: String::new(),
            detail_html: String::new(),
        }
    }

    fn mini_store(kps: Vec<Kp>, prereqs: Vec<(&str, &str)>) -> Store {
        let mut kmap = HashMap::new();
        for k in kps {
            kmap.insert(k.id.clone(), k);
        }
        for (src, dst) in prereqs {
            kmap.get_mut(dst).unwrap().prereqs.push(src.to_string());
        }
        let mut curriculum_json = serde_json::json!({
            "title": "t", "subtitle": "s", "version": "0",
            "methodology": {"intro": "i", "rules": []},
            "modules": [{"id": "m1", "num": 0, "title": "M", "icon": "x", "color": "#000", "subtitle": "s",
                "chapters": [{"id": "c1", "title": "C", "objectives": [], "kps": [], "checkpoint": []}]}]
        });
        let ids: Vec<String> = kmap.keys().cloned().collect();
        curriculum_json["modules"][0]["chapters"][0]["kps"] = serde_json::json!(ids);
        let curriculum: crate::Curriculum = serde_json::from_value(curriculum_json).unwrap();
        Store {
            curriculum,
            kps: kmap,
            placement: Default::default(),
            unplaced: vec![],
            errors: vec![],
            warnings: vec![],
            similar_edges: vec![],
            galaxy: vec![],
        }
    }

    #[test]
    fn hub_gets_inner_orbit_and_isolated_outermost() {
        // k0 是枢纽（被 5 个节点依赖），k_iso 孤立
        let kps = vec![
            kp("k0"),
            kp("k1"),
            kp("k2"),
            kp("k3"),
            kp("k4"),
            kp("k5"),
            kp("k_iso"),
        ];
        let store = mini_store(
            kps,
            vec![
                ("k0", "k1"),
                ("k0", "k2"),
                ("k0", "k3"),
                ("k0", "k4"),
                ("k0", "k5"),
            ],
        );
        let g = build_galaxy(&store);
        let find = |id: &str| g.nodes.iter().find(|n| n.id == id).unwrap();
        let hub = find("k0");
        let iso = find("k_iso");
        let leaf = find("k1");
        assert_eq!(hub.core, 5);
        assert_eq!(hub.orbit, 0, "枢纽应在最内圈");
        assert!(
            iso.orbit > hub.orbit && iso.orbit > leaf.orbit,
            "孤岛应在最外圈"
        );
        // 距心验证：枢纽半径最小
        let dist = |n: &GalaxyNode| ((n.x - 800.0).powi(2) + (n.y - 470.0).powi(2)).sqrt();
        assert!(dist(hub) < dist(leaf) && dist(leaf) < dist(iso));
    }

    #[test]
    fn layout_is_deterministic() {
        let store = mini_store(vec![kp("a"), kp("b"), kp("c")], vec![("a", "b")]);
        let g1 = build_galaxy(&store);
        let g2 = build_galaxy(&store);
        assert_eq!(g1.nodes.len(), g2.nodes.len());
        for (n1, n2) in g1.nodes.iter().zip(g2.nodes.iter()) {
            assert_eq!(n1.id, n2.id);
            assert_eq!(n1.x, n2.x);
            assert_eq!(n1.y, n2.y);
        }
    }

    #[test]
    fn uniform_orbit_radii_per_ring() {
        let store = mini_store(vec![kp("a"), kp("b"), kp("c"), kp("d")], vec![("a", "b")]);
        let g = build_galaxy(&store);
        // 同一 orbit 的节点半径应相同（正圆轨道）
        for orbit in [0u32, 1, 2] {
            let ring: Vec<f64> = g
                .nodes
                .iter()
                .filter(|n| n.orbit == orbit)
                .map(|n| ((n.x - 800.0).powi(2) + (n.y - 470.0).powi(2)).sqrt())
                .collect();
            if ring.len() > 1 {
                let (lo, hi) = ring
                    .iter()
                    .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
                assert!(hi - lo < 1.0, "orbit {orbit} 半径不一致: {lo}~{hi}");
            }
        }
    }
}
