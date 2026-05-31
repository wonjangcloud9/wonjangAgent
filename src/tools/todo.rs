//! 할 일 도구: 체크리스트를 추가/조회/완료/삭제한다.

use super::{Tool, ToolContext, ToolSpec};
use crate::todos::TodoStore;
use anyhow::{anyhow, Result};
use serde_json::Value;

pub struct AddTodoTool;

impl Tool for AddTodoTool {
    fn name(&self) -> &'static str {
        "add_todo"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "add_todo",
            description:
                "할 일(체크리스트 항목)을 추가합니다. 시각이 정해진 약속은 add_reminder를, \
                그냥 해야 할 일 목록은 이걸 쓰세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "text": { "type": "string", "description": "할 일 내용" } },
                "required": ["text"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'text' 인자가 필요합니다"))?;
        let mut store = TodoStore::load()?;
        let id = store.add(text)?;
        Ok(format!("할 일 #{id} 추가: {text}"))
    }
}

pub struct ListTodosTool;

impl Tool for ListTodosTool {
    fn name(&self) -> &'static str {
        "list_todos"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_todos",
            description: "아직 안 끝낸 할 일 목록을 반환합니다.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let store = TodoStore::load()?;
        let pending = store.pending();
        if pending.is_empty() {
            return Ok("할 일이 없습니다. 깔끔하네요!".to_string());
        }
        let mut out = String::new();
        for t in pending {
            out.push_str(&format!("#{}  {}\n", t.id, t.text));
        }
        Ok(out)
    }
}

pub struct CompleteTodoTool;

impl Tool for CompleteTodoTool {
    fn name(&self) -> &'static str {
        "complete_todo"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "complete_todo",
            description: "id로 할 일을 완료 처리합니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": "완료할 할 일 id" } },
                "required": ["id"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("'id' 인자가 필요합니다"))?;
        let mut store = TodoStore::load()?;
        if store.complete(id)? {
            Ok(format!("할 일 #{id} 완료! 👍"))
        } else {
            Ok(format!("할 일 #{id}을(를) 찾을 수 없습니다."))
        }
    }
}
