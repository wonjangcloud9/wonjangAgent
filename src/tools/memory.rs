//! 메모리 도구: 에이전트가 스스로 사실을 기억/회상한다.

use super::{Tool, ToolContext, ToolSpec};
use crate::memory::Memory;
use anyhow::Result;
use serde_json::Value;

/// 사실을 영속 메모리에 저장.
pub struct RememberTool;

impl Tool for RememberTool {
    fn name(&self) -> &'static str {
        "remember"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "remember",
            description: "사용자나 환경에 대해 앞으로도 기억해야 할 중요한 사실을 영속 \
                메모리에 저장합니다. 사용자의 선호, 자주 쓰는 경로/도구, 프로젝트 맥락 등 \
                다음 세션에도 유용한 정보를 적으세요. 일회성 정보는 저장하지 마세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "fact": {
                        "type": "string",
                        "description": "기억할 사실 한 줄. 예: '사용자는 Rust와 Python을 주로 쓴다'"
                    }
                },
                "required": ["fact"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let fact = args
            .get("fact")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'fact' 인자가 필요합니다"))?;
        let mem = Memory::load()?;
        mem.append(fact)?;
        Ok(format!("기억했습니다: {fact}"))
    }
}

/// 저장된 메모리를 회상.
pub struct RecallTool;

impl Tool for RecallTool {
    fn name(&self) -> &'static str {
        "recall"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "recall",
            description: "영속 메모리에 저장된 사실들을 모두 읽어옵니다. 사용자에 대해 \
                무엇을 기억하는지 확인할 때 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let mem = Memory::load()?;
        let content = mem.read();
        if content.trim().is_empty() {
            Ok("(저장된 메모리가 없습니다)".to_string())
        } else {
            Ok(content)
        }
    }
}
