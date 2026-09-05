//! 编程题评测引擎：临时沙箱内 rustc 编译 + 运行测试。
//!
//! 流程：用户代码 + 追加测试代码 → 合并为单个测试 crate →
//! `rustc --test --edition 2021` 编译 → 运行测试二进制 → 退出码 0 = 通过。
//! 防护：独立临时目录（用后清理）、编译与运行各 30s 超时（tokio::time::timeout）。
//! 本地教学工具定位：与 cargo/rustc 同等信任级别，不建议暴露到公网。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeOutcome {
    pub passed: bool,
    /// 编译失败时为编译器 stderr；编译成功时为测试二进制的输出
    pub output: String,
    pub duration_ms: u128,
}

struct Sandbox {
    dir: PathBuf,
}

/// 沙箱目录唯一序号（纳秒时钟在部分平台分辨率不足，并行时会撞名）
static SANDBOX_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Sandbox {
    fn create() -> std::io::Result<Self> {
        let seq = SANDBOX_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "rustway-judge-{}-{}-{seq}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn write(&self, code: &str) -> std::io::Result<PathBuf> {
        let path = self.dir.join("main.rs");
        std::fs::write(&path, code)?;
        Ok(path)
    }

    fn cleanup(self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// 把用户代码与测试代码合并为一个可 `--test` 运行的翻译单元。
fn merge(user_code: &str, test_code: &str) -> String {
    format!(
        "{user_code}\n#[cfg(test)]\nmod rustway_checker {{\n    use super::*;\n{test_code}\n}}\n"
    )
}

async fn run_with_timeout(
    cmd: &mut tokio::process::Command,
    timeout: Duration,
) -> Result<(bool, String), String> {
    let child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("进程启动失败: {e}"))?;
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Err(_) => Err("超时（30s）：循环过深或计算量过大".into()),
        Ok(Err(e)) => Err(format!("等待进程失败: {e}")),
        Ok(Ok(out)) => {
            let mut text = String::from_utf8_lossy(&out.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&out.stderr));
            Ok((out.status.success(), text))
        }
    }
}

/// 评测一段 Rust 代码：编译（含用户代码的完整性）+ 运行追加测试。
pub async fn run_rust_checks(user_code: &str, test_code: &str) -> Result<JudgeOutcome, String> {
    let started = std::time::Instant::now();
    let sandbox = Sandbox::create().map_err(|e| format!("创建沙箱失败: {e}"))?;
    let result = run_inner(&sandbox, user_code, test_code).await;
    sandbox.cleanup();
    result.map(|(passed, output)| JudgeOutcome {
        passed,
        output,
        duration_ms: started.elapsed().as_millis(),
    })
}

async fn run_inner(
    sandbox: &Sandbox,
    user_code: &str,
    test_code: &str,
) -> Result<(bool, String), String> {
    let src = sandbox
        .write(&merge(user_code, test_code))
        .map_err(|e| format!("写入沙箱失败: {e}"))?;
    let bin = sandbox.dir.join("checker");

    // 1) 编译
    let compile = run_with_timeout(
        tokio::process::Command::new("rustc")
            .arg("--test")
            .arg("--edition")
            .arg("2021")
            .arg("-O")
            .arg(&src)
            .arg("-o")
            .arg(&bin),
        Duration::from_secs(30),
    )
    .await?;
    if !compile.0 {
        return Ok((
            false,
            format!("⛔ 编译失败：\n{}", truncate(&compile.1, 4000)),
        ));
    }

    // 2) 运行测试二进制
    let mut run_cmd = tokio::process::Command::new(&bin);
    let run = run_with_timeout(&mut run_cmd, Duration::from_secs(30)).await?;
    let verdict = if run.0 {
        "✅ 测试全部通过"
    } else {
        "❌ 测试未全部通过"
    };
    Ok((run.0, format!("{verdict}\n\n{}", truncate(&run.1, 4000))))
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let mut cut = max;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        &s[..cut]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn passing_code_is_judged_pass() {
        let out = run_rust_checks(
            "fn add(a: i32, b: i32) -> i32 { a + b }",
            r#"
            #[test]
            fn checks_add() { assert_eq!(add(2, 3), 5); }
            "#,
        )
        .await
        .unwrap();
        assert!(out.passed, "output: {}", out.output);
    }

    #[tokio::test]
    async fn failing_assert_is_judged_fail() {
        let out = run_rust_checks(
            "fn add(a: i32, b: i32) -> i32 { a - b }",
            r#"
            #[test]
            fn checks_add() { assert_eq!(add(2, 3), 5); }
            "#,
        )
        .await
        .unwrap();
        assert!(!out.passed);
        assert!(out.output.contains("未全部通过"));
    }

    #[tokio::test]
    async fn compile_error_reports_diagnostics() {
        let out = run_rust_checks("fn broken() { let x: i32 = \"str\"; }", "")
            .await
            .unwrap();
        assert!(!out.passed);
        assert!(out.output.contains("编译失败"));
    }
}
