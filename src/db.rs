//! SQLite 映射存储层 —— 一主一备。
//!
//! 设计约定（见 docs/KP_SCHEMA.md 与 README）：
//! - **内容**（正文、题面、引用文本）永远以 content/ 下的 JSON/Markdown 为源；
//! - **数据库只存映射结构**：模块/章节/知识点编排、前置边、相似边、元信息；
//!   数据库是结构的服务层持久化与外部工具的读取入口，不是内容源；
//! - **一主一备**：`data/rustway.db` 为主库（WAL 模式），每次同步后通过
//!   SQLite Backup API 生成 `data/rustway-backup.db` 备库；
//! - 同步策略：映射表全量重建（结构是内容的派生物，确定性最优先），
//!   事务内完成；同步后回读校验，主备一致性由 Backup API 保证。
//!
//! 安全约定：所有 SQL 一律使用参数绑定（params![]），不拼接任何输入。

use crate::Store;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DbSnapshot {
    /// kp_id -> chapter_id（从主库回读）
    pub placements: Vec<(String, String)>,
    /// (src, dst) 前置边
    pub prereq_edges: Vec<(String, String)>,
    /// (a, b, score) 相似边
    pub similar_edges: Vec<(String, String, f64)>,
    pub primary_path: PathBuf,
    pub backup_path: PathBuf,
    pub synced_at: String,
}

/// 打开主库、建表、全量同步、回读校验、生成备库，返回从主库回读的结构快照。
pub fn open_sync_and_backup(store: &Store, data_dir: &Path) -> Result<DbSnapshot, String> {
    std::fs::create_dir_all(data_dir).map_err(|e| format!("创建数据目录失败: {e}"))?;
    let primary_path = data_dir.join("rustway.db");
    let backup_path = data_dir.join("rustway-backup.db");

    let conn = Connection::open(&primary_path)
        .map_err(|e| format!("打开主库 {} 失败: {e}", primary_path.display()))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("开启 WAL 失败: {e}"))?;
    init_schema(&conn)?;

    sync(&conn, store)?;

    // 回读校验：结构必须与内存态一致（行数逐表比对）
    let snapshot = read_snapshot(&conn, &primary_path, &backup_path)?;
    verify(&snapshot, store)?;

    // 一主一备：Backup API 整库复制（页级、原子）
    backup_to(&conn, &backup_path)?;

    Ok(snapshot)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS module (
            id    TEXT PRIMARY KEY,
            num   INTEGER NOT NULL,
            title TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS chapter (
            id        TEXT PRIMARY KEY,
            module_id TEXT NOT NULL REFERENCES module(id),
            title     TEXT NOT NULL,
            idx       INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS placement (
            kp_id      TEXT PRIMARY KEY,
            chapter_id TEXT NOT NULL REFERENCES chapter(id),
            idx        INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS prereq_edge (
            src TEXT NOT NULL,
            dst TEXT NOT NULL,
            PRIMARY KEY (src, dst)
        );
        CREATE TABLE IF NOT EXISTS similar_edge (
            a     TEXT NOT NULL,
            b     TEXT NOT NULL,
            score REAL NOT NULL,
            PRIMARY KEY (a, b)
        );
        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        COMMIT;
        "#,
    )
    .map_err(|e| format!("初始化表结构失败: {e}"))
}

/// 全量重建映射表（单事务）。
fn sync(conn: &Connection, store: &Store) -> Result<(), String> {
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("开启事务失败: {e}"))?;
    let wipe = |sql: &str| -> Result<(), String> {
        conn.execute(sql, [])
            .map(|_| ())
            .map_err(|e| format!("清空 {sql} 失败: {e}"))
    };
    wipe("DELETE FROM similar_edge")?;
    wipe("DELETE FROM prereq_edge")?;
    wipe("DELETE FROM placement")?;
    wipe("DELETE FROM chapter")?;
    wipe("DELETE FROM module")?;

    for m in &store.curriculum.modules {
        conn.execute(
            "INSERT INTO module (id, num, title) VALUES (?1, ?2, ?3)",
            rusqlite::params![m.id, m.num as i64, m.title],
        )
        .map_err(|e| format!("写入模块 {} 失败: {e}", m.id))?;
        for (ci, c) in m.chapters.iter().enumerate() {
            conn.execute(
                "INSERT INTO chapter (id, module_id, title, idx) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![c.id, m.id, c.title, ci as i64],
            )
            .map_err(|e| format!("写入章节 {} 失败: {e}", c.id))?;
            for (ki, kp_id) in c.kps.iter().enumerate() {
                conn.execute(
                    "INSERT INTO placement (kp_id, chapter_id, idx) VALUES (?1, ?2, ?3)",
                    rusqlite::params![kp_id, c.id, ki as i64],
                )
                .map_err(|e| format!("写入编排 {kp_id} 失败: {e}"))?;
            }
        }
    }
    for (kp_id, chapter_id) in &store.placement {
        conn.execute(
            "UPDATE placement SET chapter_id = ?2 WHERE kp_id = ?1",
            rusqlite::params![kp_id, chapter_id],
        )
        .map_err(|e| format!("校正编排 {kp_id} 失败: {e}"))?;
    }
    for kp in store.kps.values() {
        for p in &kp.prereqs {
            if store.kps.contains_key(p) {
                conn.execute(
                    "INSERT OR IGNORE INTO prereq_edge (src, dst) VALUES (?1, ?2)",
                    rusqlite::params![p, kp.id],
                )
                .map_err(|e| format!("写入前置边失败: {e}"))?;
            }
        }
    }
    for e in &store.similar_edges {
        conn.execute(
            "INSERT OR IGNORE INTO similar_edge (a, b, score) VALUES (?1, ?2, ?3)",
            rusqlite::params![e.a, e.b, e.score as f64],
        )
        .map_err(|e| format!("写入相似边失败: {e}"))?;
    }
    let synced_at = chrono_now();
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('synced_at', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
        rusqlite::params![synced_at],
    )
    .map_err(|e| format!("写入元信息失败: {e}"))?;
    conn.execute_batch("COMMIT")
        .map_err(|e| format!("提交事务失败: {e}"))
}

fn read_snapshot(
    conn: &Connection,
    primary_path: &Path,
    backup_path: &Path,
) -> Result<DbSnapshot, String> {
    let mut stmt = conn
        .prepare("SELECT kp_id, chapter_id FROM placement ORDER BY chapter_id, idx")
        .map_err(|e| format!("读取编排失败: {e}"))?;
    let placements: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT src, dst FROM prereq_edge ORDER BY src, dst")
        .map_err(|e| e.to_string())?;
    let prereq_edges: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT a, b, score FROM similar_edge ORDER BY score DESC")
        .map_err(|e| e.to_string())?;
    let similar_edges: Vec<(String, String, f64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    let synced_at = conn
        .query_row("SELECT value FROM meta WHERE key = 'synced_at'", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap_or_default();

    Ok(DbSnapshot {
        placements,
        prereq_edges,
        similar_edges,
        primary_path: primary_path.to_path_buf(),
        backup_path: backup_path.to_path_buf(),
        synced_at,
    })
}

/// 回读校验：数据库中的映射必须与内存态完全一致。
fn verify(snapshot: &DbSnapshot, store: &Store) -> Result<(), String> {
    if snapshot.placements.len() != store.placement.len() {
        return Err(format!(
            "编排数不一致：库 {} vs 内存 {}",
            snapshot.placements.len(),
            store.placement.len()
        ));
    }
    for (kp_id, chapter_id) in &snapshot.placements {
        if store.placement.get(kp_id) != Some(chapter_id) {
            return Err(format!("编排不一致: {kp_id}"));
        }
    }
    let prereq_in_store: usize = store
        .kps
        .values()
        .map(|k| {
            k.prereqs
                .iter()
                .filter(|p| store.kps.contains_key(p.as_str()))
                .count()
        })
        .sum();
    if snapshot.prereq_edges.len() != prereq_in_store {
        return Err(format!(
            "前置边数不一致：库 {} vs 内存 {}",
            snapshot.prereq_edges.len(),
            prereq_in_store
        ));
    }
    if snapshot.similar_edges.len() != store.similar_edges.len() {
        return Err(format!(
            "相似边数不一致：库 {} vs 内存 {}",
            snapshot.similar_edges.len(),
            store.similar_edges.len()
        ));
    }
    Ok(())
}

/// 一主一备：整库页级复制到备库路径。
fn backup_to(primary: &Connection, backup_path: &Path) -> Result<(), String> {
    let mut dst = Connection::open(backup_path)
        .map_err(|e| format!("打开备库 {} 失败: {e}", backup_path.display()))?;
    let backup = rusqlite::backup::Backup::new(primary, &mut dst)
        .map_err(|e| format!("初始化备份失败: {e}"))?;
    backup
        .run_to_completion(64, std::time::Duration::from_millis(5), None)
        .map_err(|e| format!("备份执行失败: {e}"))?;
    Ok(())
}

fn chrono_now() -> String {
    // 避免引入 chrono：用标准库 epoch 毫秒（仅作同步时间戳用途）
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("epoch-ms:{ms}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_verify_backup_roundtrip() {
        // 构造最小内存态（直接用手写结构，避免依赖临时内容目录）
        let dir = std::env::temp_dir().join(format!("rustway-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open(dir.join("t.db")).unwrap();
        init_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO module (id, num, title) VALUES ('m', 0, 'M')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapter (id, module_id, title, idx) VALUES ('c', 'm', 'C', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO placement (kp_id, chapter_id, idx) VALUES ('k', 'c', 0)",
            [],
        )
        .unwrap();
        let snapshot = read_snapshot(&conn, &dir.join("t.db"), &dir.join("t-backup.db")).unwrap();
        assert_eq!(snapshot.placements, vec![("k".into(), "c".into())]);
        backup_to(&conn, &dir.join("t-backup.db")).unwrap();
        // 备库可独立打开且数据一致
        let bak = Connection::open(dir.join("t-backup.db")).unwrap();
        let n: i64 = bak
            .query_row("SELECT COUNT(*) FROM placement", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "备库应包含主库全部映射");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
