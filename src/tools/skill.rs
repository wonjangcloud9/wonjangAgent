//! 스킬 도구: 에이전트가 절차 지식을 저장/조회/활용한다.

use super::{Tool, ToolContext, ToolSpec};
use crate::skill::SkillStore;
use anyhow::Result;
use serde_json::Value;

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("'{key}' 인자가 필요합니다"))
}

/// 재사용 가능한 스킬(절차)을 저장.
pub struct SaveSkillTool;

impl Tool for SaveSkillTool {
    fn name(&self) -> &'static str {
        "save_skill"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "save_skill",
            description: "어려운 작업을 해결한 뒤, 같은 일을 다시 할 때 바로 쓸 수 있도록 \
                재사용 가능한 '스킬'(절차/방법)을 저장합니다. 단계별 절차, 주의점, 필요한 \
                명령 등을 마크다운으로 정리해 두세요. 일회성 작업은 저장하지 마세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "스킬 이름(짧고 명확하게). 예: 'git 강제 푸시 복구'" },
                    "description": { "type": "string", "description": "이 스킬이 무엇을 하는지 한 줄 설명" },
                    "content": { "type": "string", "description": "절차 본문(마크다운). 단계, 명령, 주의점 포함" }
                },
                "required": ["name", "description", "content"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let name = arg_str(args, "name")?;
        let description = arg_str(args, "description")?;
        let content = arg_str(args, "content")?;
        let store = SkillStore::load()?;
        let existed = store.read(name).is_ok();
        let path = store.save(name, description, content)?;
        // 성장 틱: 새 스킬이면 "N개째"(사용자 화면에 성장이 보이게), 갱신이면 정직하게.
        if existed {
            Ok(format!("📚 '{name}' 스킬을 갱신했어요: {}", path.display()))
        } else {
            let n = store.list().map(|s| s.len()).unwrap_or(0);
            Ok(format!(
                "📚 '{name}' 스킬을 익혔어요({n}개째): {}",
                path.display()
            ))
        }
    }
}

/// 보유한 스킬 목록 조회.
pub struct ListSkillsTool;

impl Tool for ListSkillsTool {
    fn name(&self) -> &'static str {
        "list_skills"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_skills",
            description: "저장된 스킬(이름과 설명)의 목록을 반환합니다. 작업을 시작하기 전에 \
                관련 스킬이 있는지 확인할 때 사용하세요.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let store = SkillStore::load()?;
        let skills = store.list()?;
        if skills.is_empty() {
            return Ok("(저장된 스킬이 없습니다)".to_string());
        }
        let lines: Vec<String> = skills
            .iter()
            .map(|s| format!("- {} — {}", s.name, s.description))
            .collect();
        Ok(lines.join("\n"))
    }
}

/// 특정 스킬의 전체 절차를 읽어옴.
pub struct ReadSkillTool;

impl Tool for ReadSkillTool {
    fn name(&self) -> &'static str {
        "read_skill"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_skill",
            description: "이름으로 스킬의 전체 절차 본문을 읽어옵니다. 비슷한 작업을 \
                수행하기 전에 저장해 둔 방법을 참고할 때 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "읽어올 스킬 이름" }
                },
                "required": ["name"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let name = arg_str(args, "name")?;
        let store = SkillStore::load()?;
        store.read(name)
    }
}
