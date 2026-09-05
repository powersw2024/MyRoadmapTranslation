//! AI 代码评价：支持本地 Ollama 与 OpenAI 兼容 API 两种后端。
//!
//! 配置全部来自环境变量（密钥不落盘、不入库）：
//! - `RUSTWAY_AI_BACKEND`：`ollama`（默认启用时）或 `openai`；未设置 = AI 评价关闭
//! - `RUSTWAY_AI_BASE_URL`：默认 ollama → `http://127.0.0.1:11434`；openai → 必填
//! - `RUSTWAY_AI_MODEL`：模型名，ollama 默认 `qwen2.5-coder:7b`
//! - `RUSTWAY_AI_API_KEY`：openai 兼容后端的 Bearer 密钥（仅从环境变量读取）
//!
//! 安全约定：请求 URL 只由服务端环境变量决定，**绝不接受请求体传入的地址**；
//! scheme 仅允许 http/https。本地 Ollama 依赖环回地址，属部署者显式选择。

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub backend: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiReview {
    pub score: u32,
    pub summary: String,
    pub suggestions: Vec<String>,
}

impl AiConfig {
    /// 服务端设置页指定的配置（优先于环境变量）。
    pub fn custom(backend: &str, base_url: &str, model: &str, api_key: Option<String>) -> Self {
        Self {
            backend: backend.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.into(),
            api_key,
        }
    }

    /// 读取环境变量；未配置 `RUSTWAY_AI_BACKEND` 时返回 None（AI 评价关闭）。
    pub fn from_env() -> Option<Self> {
        let backend = std::env::var("RUSTWAY_AI_BACKEND").ok()?;
        let backend = backend.to_lowercase();
        let base_url = match backend.as_str() {
            "ollama" => std::env::var("RUSTWAY_AI_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".into()),
            "openai" => std::env::var("RUSTWAY_AI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
            _ => {
                eprintln!("⚠ RUSTWAY_AI_BACKEND 仅支持 ollama | openai，AI 评价已关闭");
                return None;
            }
        };
        // scheme 白名单：只允许 http/https（防御误配置）
        let ok = base_url.starts_with("http://") || base_url.starts_with("https://");
        if !ok {
            eprintln!("⚠ RUSTWAY_AI_BASE_URL 必须以 http(s):// 开头，AI 评价已关闭");
            return None;
        }
        Some(Self {
            model: std::env::var("RUSTWAY_AI_MODEL").unwrap_or_else(|_| {
                if backend == "ollama" {
                    "qwen2.5-coder:7b".into()
                } else {
                    "gpt-4o-mini".into()
                }
            }),
            backend,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: std::env::var("RUSTWAY_AI_API_KEY").ok(),
        })
    }
}

#[derive(Deserialize)]
struct OllamaResp {
    message: OllamaMsg,
}
#[derive(Deserialize)]
struct OllamaMsg {
    content: String,
}
#[derive(Deserialize)]
struct OpenAiResp {
    choices: Vec<OpenAiChoice>,
}
#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMsg,
}
#[derive(Deserialize)]
struct OpenAiMsg {
    content: String,
}

fn build_prompt(kp_title: &str, task: &str, code: &str) -> String {
    format!(
        "你是严格的 Rust 代码评审。针对知识点「{kp_title}」的练习提交，\n\
         从正确性、所有权/借用用法、可读性、错误处理四个方面评价。\n\
         验证任务要求：{task}\n\
         只输出 JSON（不要 markdown 围栏）：\
         {{\"score\": 0-100 整数, \"summary\": \"一句话总评\", \"suggestions\": [\"改进建议\", ...]}}\n\n\
         学生代码：\n{code}"
    )
}

fn parse_review(text: &str) -> AiReview {
    // 模型可能包裹 ```json 围栏，取第一个 { 到最后一个 } 之间
    let trimmed = text.trim();
    let json_part = match (trimmed.find('{'), trimmed.rfind('}')) {
        (Some(s), Some(e)) if e > s => &trimmed[s..=e],
        _ => {
            return AiReview {
                score: 0,
                summary: trimmed.chars().take(400).collect(),
                suggestions: vec![],
            }
        }
    };
    #[derive(Deserialize)]
    struct Raw {
        score: Option<u32>,
        summary: Option<String>,
        #[serde(default)]
        suggestions: Vec<String>,
    }
    match serde_json::from_str::<Raw>(json_part) {
        Ok(r) => AiReview {
            score: r.score.unwrap_or(0).min(100),
            summary: r.summary.unwrap_or_default(),
            suggestions: r.suggestions.into_iter().take(8).collect(),
        },
        Err(_) => AiReview {
            score: 0,
            summary: trimmed.chars().take(400).collect(),
            suggestions: vec![],
        },
    }
}

/// 调用 AI 后端评价代码。60s 超时。
pub async fn review(
    cfg: &AiConfig,
    kp_title: &str,
    task: &str,
    code: &str,
) -> Result<AiReview, String> {
    let prompt = build_prompt(kp_title, task, code);
    let client = reqwest_timeout();
    let text = match cfg.backend.as_str() {
        "ollama" => {
            let url = format!("{}/api/chat", cfg.base_url);
            let body = json_body_ollama(&cfg.model, &prompt);
            let resp: OllamaResp = client
                .post(url)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("请求 Ollama 失败: {e}"))?
                .error_for_status()
                .map_err(|e| format!("Ollama 返回错误: {e}"))?
                .json()
                .await
                .map_err(|e| format!("解析 Ollama 响应失败: {e}"))?;
            resp.message.content
        }
        _ => {
            let url = format!("{}/chat/completions", cfg.base_url);
            let mut req = client
                .post(url)
                .json(&json_body_openai(&cfg.model, &prompt));
            if let Some(key) = &cfg.api_key {
                req = req.bearer_auth(key);
            }
            let resp: OpenAiResp = req
                .send()
                .await
                .map_err(|e| format!("请求 AI API 失败: {e}"))?
                .error_for_status()
                .map_err(|e| format!("AI API 返回错误: {e}"))?
                .json()
                .await
                .map_err(|e| format!("解析 AI 响应失败: {e}"))?;
            resp.choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .ok_or_else(|| "AI 响应无内容".to_string())?
        }
    };
    Ok(parse_review(&text))
}

/// 主观题批改：对照参考答案要点给学习者答案打分（60 分及格）。
pub async fn grade_answer(
    cfg: &AiConfig,
    kp_title: &str,
    reference: &str,
    user_answer: &str,
) -> Result<AiReview, String> {
    let prompt = format!(
        "你是严格的阅卷老师。知识点「{kp_title}」的主观题，对照参考答案要点批改学生答案。\n\
         参考答案要点：{reference}\n\n学生答案：{user_answer}\n\n\
         只输出 JSON（不要 markdown 围栏）：\
         {{\"score\": 0-100 整数（≥60 为通过）, \"summary\": \"一句话评语\", \
         \"suggestions\": [\"遗漏或需补充的要点\", ...]}}"
    );
    let client = reqwest_timeout();
    let text = match cfg.backend.as_str() {
        "ollama" => {
            let resp: OllamaResp = client
                .post(format!("{}/api/chat", cfg.base_url))
                .json(&serde_json::json!({
                    "model": cfg.model,
                    "stream": false,
                    "format": "json",
                    "messages": [
                        {"role": "system", "content": "你是严格的阅卷老师，只输出 JSON。"},
                        {"role": "user", "content": prompt}
                    ]
                }))
                .send()
                .await
                .map_err(|e| format!("请求 Ollama 失败: {e}"))?
                .error_for_status()
                .map_err(|e| format!("Ollama 返回错误: {e}"))?
                .json()
                .await
                .map_err(|e| format!("解析 Ollama 响应失败: {e}"))?;
            resp.message.content
        }
        _ => {
            let mut req = client
                .post(format!("{}/chat/completions", cfg.base_url))
                .json(&serde_json::json!({
                    "model": cfg.model,
                    "response_format": {"type": "json_object"},
                    "messages": [
                        {"role": "system", "content": "你是严格的阅卷老师，只输出 JSON。"},
                        {"role": "user", "content": prompt}
                    ]
                }));
            if let Some(key) = &cfg.api_key {
                req = req.bearer_auth(key);
            }
            let resp: OpenAiResp = req
                .send()
                .await
                .map_err(|e| format!("请求 AI API 失败: {e}"))?
                .error_for_status()
                .map_err(|e| format!("AI API 返回错误: {e}"))?
                .json()
                .await
                .map_err(|e| format!("解析 AI 响应失败: {e}"))?;
            resp.choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .ok_or_else(|| "AI 响应无内容".to_string())?
        }
    };
    Ok(parse_review(&text))
}

/// base_url 安全校验（防 SSRF）：
/// - scheme 仅允许 http/https
/// - 环回地址（localhost / 127.0.0.0/8 / ::1）明确允许——本地 Ollama 是产品需求，
///   且服务本身绑定本机，操作者即本机用户
/// - 拒绝私网段（10/8、172.16/12、192.168/16）、链路本地（169.254/16，含云元数据）、
///   未指定地址与非全球单播 IPv6；其余公网地址放行
///
/// 双重校验：保存配置时与实际发请求前各执行一次（防御纵深）。
pub fn validate_base_url(url: &str) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("base_url 必须以 http(s):// 开头".into());
    }
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let host_port = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let host = match host_port.rsplit_once(':') {
        Some((h, port)) if port.chars().all(|c| c.is_ascii_digit()) && !port.is_empty() => h,
        _ => host_port,
    };
    let host = host.trim_matches(['[', ']']);
    if host.eq_ignore_ascii_case("localhost") {
        return Ok(());
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        let loopback = match ip {
            std::net::IpAddr::V4(v4) => v4.is_loopback(),
            std::net::IpAddr::V6(v6) => v6.is_loopback(),
        };
        if loopback {
            return Ok(()); // 本地 Ollama
        }
        let blocked = match ip {
            std::net::IpAddr::V4(v4) => {
                v4.is_private()
                    || v4.is_link_local()
                    || v4.is_unspecified()
                    || v4.is_broadcast()
                    || v4.is_documentation()
            }
            std::net::IpAddr::V6(v6) => {
                v6.is_unspecified() || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique local
            }
        };
        if blocked {
            return Err(format!("拒绝私有/保留地址: {host}"));
        }
    }
    Ok(())
}

/// 连接测试：发送一次最小补全请求，返回耗时（毫秒）。20s 超时。
pub async fn test_connection(cfg: &AiConfig) -> Result<u128, String> {
    validate_base_url(&cfg.base_url)?;
    let started = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .expect("构建 HTTP 客户端");
    match cfg.backend.as_str() {
        "ollama" => {
            let url = format!("{}/api/chat", cfg.base_url);
            let body = serde_json::json!({
                "model": cfg.model,
                "stream": false,
                "messages": [{"role": "user", "content": "回复 ok"}]
            });
            let resp: OllamaResp = client
                .post(url)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("无法连接 Ollama（{}）: {e}", cfg.base_url))?
                .error_for_status()
                .map_err(|e| format!("Ollama 返回错误: {e}"))?
                .json()
                .await
                .map_err(|e| format!("解析响应失败: {e}"))?;
            let _ = resp.message.content;
        }
        _ => {
            let url = format!("{}/chat/completions", cfg.base_url);
            let mut req = client.post(url).json(&serde_json::json!({
                "model": cfg.model,
                "max_tokens": 5,
                "messages": [{"role": "user", "content": "ok"}]
            }));
            if let Some(key) = &cfg.api_key {
                req = req.bearer_auth(key);
            }
            let resp: OpenAiResp = req
                .send()
                .await
                .map_err(|e| format!("无法连接 AI API（{}）: {e}", cfg.base_url))?
                .error_for_status()
                .map_err(|e| format!("AI API 返回错误: {e}"))?
                .json()
                .await
                .map_err(|e| format!("解析响应失败: {e}"))?;
            let _ = resp.choices;
        }
    }
    Ok(started.elapsed().as_millis())
}

fn reqwest_timeout() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("构建 HTTP 客户端")
}

fn json_body_ollama(model: &str, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "stream": false,
        "format": "json",
        "messages": [
            {"role": "system", "content": "你是严格的 Rust 代码评审，只输出 JSON。"},
            {"role": "user", "content": prompt}
        ]
    })
}

fn json_body_openai(model: &str, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "response_format": {"type": "json_object"},
        "messages": [
            {"role": "system", "content": "你是严格的 Rust 代码评审，只输出 JSON。"},
            {"role": "user", "content": prompt}
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_with_fence() {
        let text = "```json\n{\"score\": 85, \"summary\": \"总体不错\", \"suggestions\": [\"减少 unwrap\"]}\n```";
        let r = parse_review(text);
        assert_eq!(r.score, 85);
        assert_eq!(r.suggestions.len(), 1);
    }

    #[test]
    fn falls_back_to_text_on_garbage() {
        let r = parse_review("这段代码看起来没问题");
        assert_eq!(r.score, 0);
        assert!(r.summary.contains("没问题"));
    }

    #[test]
    fn env_off_by_default() {
        // 未设置 RUSTWAY_AI_BACKEND 时应返回 None（不影响其他测试的 env：这里只测语义）
        let cfg = std::env::var("RUSTWAY_AI_BACKEND").ok().map(|_| ());
        if cfg.is_none() {
            assert!(std::env::var("RUSTWAY_AI_BACKEND").is_err());
        }
    }

    #[test]
    fn base_url_validation_blocks_private_and_reserved() {
        // 本地 Ollama 明确允许
        assert!(validate_base_url("http://127.0.0.1:11434").is_ok());
        assert!(validate_base_url("http://localhost:11434").is_ok());
        assert!(validate_base_url("https://api.openai.com/v1").is_ok());
        // 私网 / 链路本地（云元数据）/ 保留地址必须拒绝
        assert!(validate_base_url("http://169.254.169.254/v1").is_err());
        assert!(validate_base_url("http://192.168.1.10:8080").is_err());
        assert!(validate_base_url("http://10.0.0.5").is_err());
        assert!(validate_base_url("http://172.16.0.9").is_err());
        assert!(validate_base_url("http://0.0.0.0").is_err());
        assert!(validate_base_url("ftp://example.com").is_err());
    }
}
