//! 서브에이전트 도구: 큰 작업을 격리된 하위 에이전트에 위임한다.
//!
//! 상위 에이전트가 독립적인 하위 작업을 새 에이전트 인스턴스(자체 대화 맥락)에
//! 맡기고, 그 결과만 회수한다. 여러 하위 작업을 병렬로 돌릴 수도 있어, 긴
//! 작업을 잘게 나눠 동시에 처리하면서 상위 컨텍스트는 깔끔하게 유지한다.

use super::{subagent_tools, Tool, ToolContext, ToolSpec};
use crate::agent;
use crate::config::Config;
use crate::llm::{LlmClient, Message};
use crate::ui;
use anyhow::Result;
use serde_json::Value;

/// 서브에이전트용 시스템 프롬프트.
fn subagent_prompt() -> String {
    let base = agent::system_prompt(None, None);
    format!(
        "{base}\n\
         당신은 상위 에이전트가 위임한 '하위 작업'을 수행하는 보조 에이전트입니다.\n\
         - 주어진 작업만 끝까지 처리하고, 결과를 한국어로 간결히 보고하세요.\n\
         - 추가 질문 없이, 가진 도구로 스스로 끝내세요.\n"
    )
}

/// 하위 작업 하나를 실행하고 최종 답변을 반환한다.
async fn run_subagent(
    cfg: Config,
    task: String,
    auto_approve: bool,
    allow_dangerous: bool,
) -> Result<String> {
    let client = LlmClient::new(cfg.base_url.clone(), cfg.api_key.clone(), cfg.model.clone());
    let tools = subagent_tools();
    let ctx = ToolContext {
        auto_approve,
        allow_dangerous,
    };
    let mut messages = vec![
        Message::system(subagent_prompt()),
        Message::user(task),
    ];
    let answer = agent::run_turn(&client, &cfg, &tools, &ctx, &mut messages).await?;
    Ok(answer.unwrap_or_else(|| "(서브에이전트가 답변을 내지 못했습니다)".to_string()))
}

/// 단일 서브에이전트.
pub struct SpawnSubagentTool;

impl Tool for SpawnSubagentTool {
    fn name(&self) -> &'static str {
        "spawn_subagent"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "spawn_subagent",
            description: "독립적인 하위 작업을 별도 에이전트에 위임하고 그 결과만 받아옵니다. \
                긴 조사·정리 작업을 깔끔히 분리할 때 유용합니다. 작업 설명은 그 자체로 \
                완결되도록(필요한 맥락 포함) 구체적으로 적으세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "task": { "type": "string", "description": "위임할 하위 작업(자기완결적으로 구체적으로)" }
                },
                "required": ["task"]
            }),
        }
    }

    fn execute(&self, args: &Value, ctx: &ToolContext) -> Result<String> {
        let task = args
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'task' 인자가 필요합니다"))?
            .to_string();
        let auto = ctx.auto_approve;
        let danger = ctx.allow_dangerous;
        ui::tool_result(&format!("서브에이전트 시작: {}", first_line(&task)));
        crate::util::run_async(async move {
            let cfg = Config::load()?;
            run_subagent(cfg, task, auto, danger).await
        })
    }
}

/// 여러 서브에이전트를 병렬 실행.
pub struct SpawnSubagentsTool;

impl Tool for SpawnSubagentsTool {
    fn name(&self) -> &'static str {
        "spawn_subagents"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "spawn_subagents",
            description: "여러 하위 작업을 동시에(병렬로) 각각의 서브에이전트에 맡기고 모든 \
                결과를 모아 반환합니다. 서로 독립적인 작업 여러 개를 빠르게 처리할 때 쓰세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "동시에 처리할 하위 작업 목록(각각 자기완결적으로)"
                    }
                },
                "required": ["tasks"]
            }),
        }
    }

    fn execute(&self, args: &Value, ctx: &ToolContext) -> Result<String> {
        let tasks: Vec<String> = args
            .get("tasks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("'tasks' 배열이 필요합니다"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        if tasks.is_empty() {
            return Ok("실행할 하위 작업이 없습니다.".to_string());
        }
        let auto = ctx.auto_approve;
        let danger = ctx.allow_dangerous;
        ui::tool_result(&format!("서브에이전트 {}개 병렬 시작", tasks.len()));

        crate::util::run_async(async move {
            let cfg = Config::load()?;
            // 각 하위 작업을 tokio 태스크로 spawn → 동시에 진행.
            let mut handles = Vec::new();
            for (i, task) in tasks.into_iter().enumerate() {
                let cfg = cfg.clone();
                handles.push(tokio::spawn(async move {
                    (i + 1, run_subagent(cfg, task, auto, danger).await)
                }));
            }

            let mut out = String::new();
            for h in handles {
                let (n, res) = h.await.unwrap_or((0, Ok("(태스크 패닉)".to_string())));
                let body = match res {
                    Ok(text) => text,
                    Err(e) => format!("(오류: {e})"),
                };
                out.push_str(&format!("── 하위 작업 #{n} 결과 ──\n{body}\n\n"));
            }
            Ok(out.trim_end().to_string())
        })
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").chars().take(70).collect()
}
