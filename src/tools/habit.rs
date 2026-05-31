//! 습관 도구: 습관 추가/체크/조회.

use super::{Tool, ToolContext, ToolSpec};
use crate::habits::{self, HabitStore};
use anyhow::{anyhow, Result};
use serde_json::Value;

pub struct AddHabitTool;

impl Tool for AddHabitTool {
    fn name(&self) -> &'static str {
        "add_habit"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "add_habit",
            description: "매일 챙길 습관을 추가합니다(예: 운동, 독서, 영어공부).",
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "name": { "type": "string", "description": "습관 이름" } },
                "required": ["name"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'name' 인자가 필요합니다"))?;
        let mut store = HabitStore::load()?;
        let id = store.add(name)?;
        Ok(format!("습관 #{id} 추가: {name}"))
    }
}

pub struct CheckHabitTool;

impl Tool for CheckHabitTool {
    fn name(&self) -> &'static str {
        "check_habit"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "check_habit",
            description:
                "오늘 습관을 완료 처리합니다(이름 또는 id로). 연속 일수를 함께 반환합니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "habit": { "type": "string", "description": "습관 이름 또는 id" } },
                "required": ["habit"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let key = args
            .get("habit")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'habit' 인자가 필요합니다"))?;
        let mut store = HabitStore::load()?;
        match store.check(key)? {
            Some((name, streak)) => Ok(format!("'{name}' 완료! 🔥 {streak}일 연속")),
            None => Ok(format!("'{key}' 습관을 찾을 수 없습니다.")),
        }
    }
}

pub struct ListHabitsTool;

impl Tool for ListHabitsTool {
    fn name(&self) -> &'static str {
        "list_habits"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_habits",
            description: "습관 목록을 오늘 완료 여부와 연속 일수와 함께 반환합니다.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let store = HabitStore::load()?;
        if store.items.is_empty() {
            return Ok("등록된 습관이 없습니다.".to_string());
        }
        let today = habits::today();
        let today_s = habits::today_str();
        let mut out = String::new();
        for h in &store.items {
            let mark = if h.done_today(&today_s) { "✓" } else { "✗" };
            out.push_str(&format!(
                "[{mark} 오늘] {} — 🔥{}일 연속\n",
                h.name,
                h.streak(today)
            ));
        }
        Ok(out)
    }
}
