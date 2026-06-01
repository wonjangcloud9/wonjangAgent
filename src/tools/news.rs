//! 뉴스 도구: 구글뉴스(한국) 헤드라인.

use super::{Tool, ToolContext, ToolSpec};
use crate::news;
use anyhow::Result;
use serde_json::Value;

pub struct NewsTool;

impl Tool for NewsTool {
    fn name(&self) -> &'static str {
        "news_headlines"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "news_headlines",
            description: "최신 뉴스 헤드라인을 가져옵니다(구글뉴스 한국). 검색어를 주면 그 \
                주제로, 없으면 주요 뉴스를 반환합니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "검색어(선택). 없으면 주요 뉴스" }
                }
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let query = args.get("query").and_then(|v| v.as_str()).map(String::from);
        let list =
            crate::util::run_async(async move { news::headlines(query.as_deref(), 8).await })?;
        if list.is_empty() {
            return Ok("뉴스를 가져오지 못했습니다.".to_string());
        }
        Ok(list
            .iter()
            .map(|h| format!("- {h}"))
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
