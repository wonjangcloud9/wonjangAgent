//! 클립보드 도구: 복사한 내용을 읽거나 결과를 클립보드에 넣는다.

use super::{Tool, ToolContext, ToolSpec};
use crate::clipboard;
use anyhow::Result;
use serde_json::Value;

/// 클립보드 읽기.
pub struct ReadClipboardTool;

impl Tool for ReadClipboardTool {
    fn name(&self) -> &'static str {
        "read_clipboard"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_clipboard",
            description: "사용자가 방금 복사한 클립보드 내용을 읽어옵니다. '복사한 거', '방금 \
                복사한 텍스트/링크'를 다뤄 달라고 하면 이걸로 가져오세요.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let content = clipboard::read()?;
        if content.trim().is_empty() {
            Ok("(클립보드가 비어 있습니다)".to_string())
        } else {
            Ok(content)
        }
    }
}

/// 클립보드 쓰기.
pub struct WriteClipboardTool;

impl Tool for WriteClipboardTool {
    fn name(&self) -> &'static str {
        "write_clipboard"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_clipboard",
            description: "결과 텍스트를 클립보드에 복사합니다. 번역·정리한 내용을 사용자가 \
                바로 붙여넣을 수 있게 할 때 쓰세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "text": { "type": "string", "description": "클립보드에 넣을 내용" } },
                "required": ["text"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'text' 인자가 필요합니다"))?;
        clipboard::write(text)?;
        Ok(format!(
            "클립보드에 복사했습니다({}자).",
            text.chars().count()
        ))
    }
}
