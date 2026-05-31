//! 도구(tool) 레지스트리.
//!
//! 에이전트가 로컬 환경을 다루기 위한 도구들을 정의한다. 각 도구는 LLM에
//! 전달할 JSON 스키마(`spec`)와 실제 실행 로직(`execute`)을 제공한다.

pub mod fs;
pub mod shell;

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
}

/// 모든 도구가 구현하는 트레이트.
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn name(&self) -> &'static str;
    /// 인자(JSON)를 받아 실행하고, 결과 문자열을 반환한다.
    fn execute(&self, args: &Value, ctx: &ToolContext) -> anyhow::Result<String>;
}

/// 기본 도구 모음을 반환한다.
pub fn default_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(shell::ShellTool),
        Box::new(fs::ReadFileTool),
        Box::new(fs::WriteFileTool),
        Box::new(fs::ListDirTool),
    ]
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
