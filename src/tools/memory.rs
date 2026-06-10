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
        let fact = &crate::memory::normalize(fact);
        let mem = Memory::load()?;
        let before = mem.count();
        mem.append(fact)?;
        let after = mem.count();
        // 성장 틱: 새로 배웠을 때만 "N개째" — 사용자 화면(ui::tool_result 첫 줄)에
        // '쓸수록 성장'이 보이게 한다. 중복이면 정직하게(모델 피드백도 정확해짐).
        if after > before {
            Ok(format!("🌱 기억했어요({after}개째): {fact}"))
        } else {
            Ok(format!("이미 기억하고 있어요: {fact}"))
        }
    }
}

/// 잘못 기억된 사실을 지운다(기억 위생 — 틀린 기억이 프롬프트를 영원히 오염시키지 않게).
pub struct ForgetTool;

impl Tool for ForgetTool {
    fn name(&self) -> &'static str {
        "forget"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "forget",
            description: "영속 메모리에서 키워드가 포함된 사실을 지웁니다. 사용자가 \
                '그거 잊어줘'·'그건 이제 아니야'라고 하거나 기억이 사실과 달라졌을 때 \
                사용하세요. 키워드가 든 모든 사실이 지워지므로 구체적인 키워드를 쓰세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "지울 기억에 포함된 키워드(구체적일수록 안전). 예: '아침형'"
                    }
                },
                "required": ["keyword"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let keyword = args
            .get("keyword")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'keyword' 인자가 필요합니다"))?;
        let mem = Memory::load()?;
        let removed = mem.forget(keyword)?;
        if removed.is_empty() {
            Ok(format!("'{keyword}'가 든 기억이 없어요."))
        } else {
            Ok(format!(
                "🗑 잊었어요({}건): {}",
                removed.len(),
                removed.join(" / ")
            ))
        }
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
