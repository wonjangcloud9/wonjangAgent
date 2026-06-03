//! LLM 클라이언트 — OpenAI 호환 채팅 완성 API.
//!
//! OpenRouter, OpenAI, DeepSeek, 로컬 vLLM 등 OpenAI 호환 엔드포인트를 모두
//! 지원해 헤르메스 에이전트처럼 "제공자 무관"을 달성한다.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 대화 메시지(요청/응답 공용).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    /// 도구 실행 결과 메시지.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// 모델이 요청한 도구 호출.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type", default = "default_tool_type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

fn default_tool_type() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    /// 인자(JSON 문자열).
    pub arguments: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

/// LLM 클라이언트.
pub struct LlmClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl LlmClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
        }
    }

    /// 채팅 완성을 1회 호출한다. `tools`는 OpenAI 형식의 도구 배열.
    pub async fn chat(&self, messages: &[Message], tools: &Value) -> Result<Message> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .header(
                "HTTP-Referer",
                "https://github.com/wonjangcloud9/wonjangAgent",
            )
            .header("X-Title", "wonjang-agent")
            .json(&body)
            .send()
            .await
            .context("LLM 요청 전송에 실패했습니다(네트워크/엔드포인트 확인)")?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("LLM 응답 오류 ({}): {}", status, truncate(&text, 500));
        }

        let parsed: ChatResponse = serde_json::from_str(&text)
            .with_context(|| format!("LLM 응답 파싱 실패: {}", truncate(&text, 500)))?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .context("LLM 응답에 choices가 없습니다")?;
        Ok(choice.message)
    }
}

fn truncate(s: &str, n: usize) -> String {
    // 한글·이모지 경계에서 패닉하지 않도록 문자 경계로 안전하게 자른다.
    let (cut, truncated) = crate::util::truncate_bytes(s, n);
    if truncated {
        format!("{cut}…")
    } else {
        cut.to_string()
    }
}
