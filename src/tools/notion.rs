//! 노션 도구: 워크스페이스 검색 / 페이지에 기록.

use super::{Tool, ToolContext, ToolSpec};
use crate::config::Config;
use crate::notion;
use anyhow::{anyhow, Result};
use serde_json::Value;

fn token() -> Result<String> {
    let cfg = Config::load()?;
    if cfg.notion_token.trim().is_empty() {
        return Err(anyhow!(
            "노션 토큰이 설정되지 않았습니다. 환경 변수 WONJANG_NOTION_TOKEN 에 통합 토큰을 \
             설정하고, 대상 페이지/DB의 연결(Connections)에 그 통합을 추가하세요."
        ));
    }
    Ok(cfg.notion_token)
}

/// 노션 검색.
pub struct NotionSearchTool;

impl Tool for NotionSearchTool {
    fn name(&self) -> &'static str {
        "notion_search"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "notion_search",
            description: "노션 워크스페이스에서 페이지/데이터베이스를 검색해 제목·URL·id를 \
                반환합니다. 페이지에 기록하려면 먼저 이걸로 대상 page_id를 찾으세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "검색어" },
                    "limit": { "type": "integer", "description": "최대 결과 수(기본 10)" }
                },
                "required": ["query"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let token = token()?;
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'query' 인자가 필요합니다"))?
            .to_string();
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let hits =
            crate::util::run_async(async move { notion::search(&token, &query, limit).await })?;
        if hits.is_empty() {
            return Ok(
                "검색 결과가 없습니다(통합이 해당 페이지에 연결되어 있는지 확인하세요)."
                    .to_string(),
            );
        }
        let mut out = String::new();
        for h in &hits {
            out.push_str(&format!(
                "[{}] {}\n  id: {}\n  {}\n",
                h.kind, h.title, h.id, h.url
            ));
        }
        Ok(out)
    }
}

/// 노션 페이지에 기록.
pub struct NotionAppendTool;

impl Tool for NotionAppendTool {
    fn name(&self) -> &'static str {
        "notion_append"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "notion_append",
            description: "노션 페이지에 단락(문단)을 덧붙입니다. page_id는 notion_search로 \
                먼저 찾으세요. 메모·일지·할 일을 노션에 기록할 때 사용합니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "대상 페이지 id" },
                    "text": { "type": "string", "description": "덧붙일 텍스트" }
                },
                "required": ["page_id", "text"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let token = token()?;
        let page_id = args
            .get("page_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'page_id' 인자가 필요합니다"))?
            .to_string();
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'text' 인자가 필요합니다"))?
            .to_string();

        crate::util::run_async(
            async move { notion::append_paragraph(&token, &page_id, &text).await },
        )?;
        Ok("노션 페이지에 기록했습니다.".to_string())
    }
}
