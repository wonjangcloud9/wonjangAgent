//! 도구(tool) 레지스트리.
//!
//! 에이전트가 로컬 환경을 다루기 위한 도구들을 정의한다. 각 도구는 LLM에
//! 전달할 JSON 스키마(`spec`)와 실제 실행 로직(`execute`)을 제공한다.

pub mod airquality;
pub mod clipboard;
pub mod coin;
pub mod dday;
pub mod exchange;
pub mod expense;
pub mod fs;
pub mod habit;
pub mod lotto;
pub mod mcp;
pub mod memory;
pub mod news;
pub mod notes;
pub mod notion;
pub mod reminder;
pub mod shell;
pub mod skill;
pub mod subagent;
pub mod subway;
pub mod todo;
pub mod weather;
pub mod web;

use serde_json::Value;

/// LLM의 function-calling 형식에 맞는 도구 명세.
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    /// JSON Schema(파라미터).
    pub parameters: Value,
}

/// 도구 실행 컨텍스트 — 사용자 승인 정책 등을 담는다.
pub struct ToolContext {
    /// true이면 위험 작업(쉘 실행 등)을 묻지 않고 자동 승인한다.
    pub auto_approve: bool,
    /// true이면 위험 명령(rm -rf 등)도 허용한다. 무인 모드 기본 차단 해제용.
    pub allow_dangerous: bool,
}

/// 모든 도구가 구현하는 트레이트.
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn name(&self) -> &'static str;
    /// 인자(JSON)를 받아 실행하고, 결과 문자열을 반환한다.
    fn execute(&self, args: &Value, ctx: &ToolContext) -> anyhow::Result<String>;
}

/// 서브에이전트가 쓰는 도구 모음(spawn 계열 제외 → 무한 재귀 방지).
pub fn subagent_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(shell::ShellTool),
        Box::new(fs::ReadFileTool),
        Box::new(fs::WriteFileTool),
        Box::new(fs::ListDirTool),
        Box::new(memory::RememberTool),
        Box::new(memory::RecallTool),
        Box::new(memory::ForgetTool),
        Box::new(skill::SaveSkillTool),
        Box::new(skill::ListSkillsTool),
        Box::new(skill::ReadSkillTool),
        Box::new(web::WebSearchTool),
        Box::new(web::WebFetchTool),
        Box::new(notes::NoteSearchTool),
        Box::new(notes::NoteReadTool),
        Box::new(notes::NoteAppendTool),
        Box::new(notes::NoteListTool),
        Box::new(reminder::AddReminderTool),
        Box::new(reminder::ListRemindersTool),
        Box::new(reminder::RemoveReminderTool),
        Box::new(todo::AddTodoTool),
        Box::new(todo::ListTodosTool),
        Box::new(todo::CompleteTodoTool),
        Box::new(clipboard::ReadClipboardTool),
        Box::new(clipboard::WriteClipboardTool),
        Box::new(dday::AddDdayTool),
        Box::new(dday::ListDdaysTool),
        Box::new(notion::NotionSearchTool),
        Box::new(notion::NotionAppendTool),
        Box::new(expense::AddExpenseTool),
        Box::new(expense::ExpenseSummaryTool),
        Box::new(habit::AddHabitTool),
        Box::new(habit::CheckHabitTool),
        Box::new(habit::ListHabitsTool),
        Box::new(subway::SubwayTool),
        Box::new(weather::WeatherTool),
        Box::new(airquality::AirQualityTool),
        Box::new(exchange::ExchangeTool),
        Box::new(coin::CoinTool),
        Box::new(news::NewsTool),
        Box::new(lotto::LottoTool),
    ]
}

/// 기본 도구 모음(서브에이전트 도구 + spawn 계열).
pub fn default_tools() -> Vec<Box<dyn Tool>> {
    let mut tools = subagent_tools();
    tools.push(Box::new(subagent::SpawnSubagentTool));
    tools.push(Box::new(subagent::SpawnSubagentsTool));
    tools
}

/// 도구 목록을 OpenAI 호환 `tools` 배열(JSON)로 직렬화한다.
pub fn tools_json(tools: &[Box<dyn Tool>]) -> Value {
    Value::Array(
        tools
            .iter()
            .map(|t| {
                let spec = t.spec();
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": spec.name,
                        "description": spec.description,
                        "parameters": spec.parameters,
                    }
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_spawn_but_subagent_does_not() {
        let default_names: Vec<&str> = default_tools().iter().map(|t| t.name()).collect();
        let sub_names: Vec<&str> = subagent_tools().iter().map(|t| t.name()).collect();

        // 기본 도구에는 spawn 계열이 있다.
        assert!(default_names.contains(&"spawn_subagent"));
        assert!(default_names.contains(&"spawn_subagents"));

        // 서브에이전트 도구에는 spawn 계열이 없어야 한다(무한 재귀 방지).
        assert!(!sub_names.contains(&"spawn_subagent"));
        assert!(!sub_names.contains(&"spawn_subagents"));

        // 기본 = 서브에이전트 + spawn 2종.
        assert_eq!(default_names.len(), sub_names.len() + 2);
    }
}
