//! 웹 도구: 검색과 페이지 가져오기.

use super::{Tool, ToolContext, ToolSpec};
use crate::web;
use anyhow::Result;
use serde_json::Value;

/// 웹 검색.
pub struct WebSearchTool;

impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_search",
            description: "웹을 검색해 상위 결과(제목·URL·요약)를 반환합니다. 최신 정보나 \
                모르는 사실을 확인할 때 사용하세요. 한국어 질의도 가능합니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "검색어" },
                    "limit": { "type": "integer", "description": "가져올 결과 수(기본 5, 최대 10)" }
                },
                "required": ["query"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'query' 인자가 필요합니다"))?
            .to_string();
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .clamp(1, 10) as usize;

        let results = crate::util::run_async(async move { web::search(&query, limit).await })?;
        if results.is_empty() {
            return Ok("검색 결과를 찾지 못했습니다(검색 엔진이 일시적으로 막혔을 수 있습니다).".to_string());
        }
        let mut out = String::new();
        for (i, r) in results.iter().enumerate() {
            out.push_str(&format!("{}. {}\n   {}\n", i + 1, r.title, r.url));
            if !r.snippet.is_empty() {
                out.push_str(&format!("   {}\n", r.snippet));
            }
        }
        Ok(out)
    }
}

/// 웹 페이지 가져오기(텍스트 추출).
pub struct WebFetchTool;

impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "web_fetch"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_fetch",
            description: "URL의 웹 페이지를 가져와 본문 텍스트를 반환합니다(HTML 태그 제거). \
                검색 결과의 링크를 자세히 읽을 때 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "가져올 페이지의 전체 URL" }
                },
                "required": ["url"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'url' 인자가 필요합니다"))?
            .to_string();
        crate::util::run_async(async move { web::fetch(&url, 12000).await })
    }
}
