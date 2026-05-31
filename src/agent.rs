//! 에이전트 루프.
//!
//! 사용자 요청 → LLM 호출 → (도구 호출이 있으면) 도구 실행 → 결과를 다시
//! LLM에 전달 → 도구 호출이 없을 때까지 반복. 헤르메스 에이전트의 핵심
//! 에이전트 루프를 러스트로 구현한 것.

use crate::config::Config;
use crate::llm::{LlmClient, Message};
use crate::tools::{tools_json, Tool, ToolContext};
use crate::ui;
use anyhow::Result;
use owo_colors::OwoColorize;
use serde_json::Value;

/// 한국어 시스템 프롬프트.
///
/// `memory_block`은 학습된 사실, `skills_block`은 보유 스킬 목록을 주입한다.
pub fn system_prompt(memory_block: Option<String>, skills_block: Option<String>) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(알 수 없음)".to_string());
    let os = std::env::consts::OS;
    let mut prompt = format!(
        "당신은 '원장'이라는 이름의 자율 AI 에이전트입니다. 사용자의 로컬 컴퓨터 환경을 \
         직접 다루어 작업을 수행합니다.\n\n\
         원칙:\n\
         - 항상 한국어로, 간결하고 친근하게 답합니다.\n\
         - 추측하지 말고 도구로 확인하세요. 파일을 보려면 read_file, 디렉터리는 list_dir, \
         시스템 작업은 run_shell을 사용합니다.\n\
         - 파괴적이거나 되돌리기 어려운 작업(삭제, 덮어쓰기, 외부 전송)은 먼저 사용자에게 \
         이유와 함께 알리고 신중히 진행하세요.\n\
         - 사용자/환경에 대해 앞으로도 유용할 사실을 알게 되면 remember 도구로 기억하세요.\n\
         - 까다로운 작업을 해결한 뒤 재사용할 만한 절차는 save_skill로 저장하고, 비슷한 \
         작업 전에는 관련 스킬을 read_skill로 펼쳐 참고하세요.\n\
         - 작업을 마치면 무엇을 했고 결과가 어떤지 한국어로 요약합니다.\n\n\
         실행 환경:\n\
         - 운영체제: {os}\n\
         - 현재 작업 디렉터리: {cwd}\n"
    );
    for block in [memory_block, skills_block].into_iter().flatten() {
        prompt.push('\n');
        prompt.push_str(&block);
        prompt.push('\n');
    }
    prompt
}

/// 한 번의 사용자 요청을 처리하는 에이전트 루프.
///
/// `messages`는 누적 대화 기록(REPL에서 재사용). 함수가 끝나면 모델의 최종
/// 답변까지 포함된 상태가 된다.
pub async fn run_turn(
    client: &LlmClient,
    cfg: &Config,
    tools: &[Box<dyn Tool>],
    ctx: &ToolContext,
    messages: &mut Vec<Message>,
) -> Result<()> {
    let tools_spec = tools_json(tools);

    for _step in 0..cfg.max_steps {
        let reply = client.chat(messages, &tools_spec).await?;

        // 도구 호출이 있으면 실행하고 결과를 대화에 추가.
        if let Some(tool_calls) = reply.tool_calls.clone() {
            if !tool_calls.is_empty() {
                // 모델의 도구 호출 메시지를 먼저 기록.
                messages.push(reply.clone());

                for call in tool_calls {
                    let args: Value = serde_json::from_str(&call.function.arguments)
                        .unwrap_or(Value::Object(Default::default()));
                    let summary = arg_summary(&call.function.name, &args);
                    ui::tool_call(&call.function.name, &summary);

                    let result = execute_tool(tools, &call.function.name, &args, ctx);
                    let result_text = match result {
                        Ok(text) => {
                            ui::tool_result(&first_line(&text));
                            text
                        }
                        Err(e) => {
                            let msg = format!("도구 실행 오류: {e}");
                            ui::tool_result(&msg);
                            msg
                        }
                    };
                    messages.push(Message::tool(call.id, result_text));
                }
                // 도구 결과를 반영해 다시 모델 호출.
                continue;
            }
        }

        // 도구 호출이 없으면 최종 답변.
        if let Some(content) = &reply.content {
            println!("\n{} {}\n", ui::agent_label(), content);
        }
        messages.push(reply);
        return Ok(());
    }

    ui::note(&format!(
        "최대 단계({})에 도달해 멈췄습니다. 작업이 복잡하면 더 작게 나눠 다시 요청해 주세요.",
        cfg.max_steps
    ));
    Ok(())
}

/// 이름으로 도구를 찾아 실행한다.
fn execute_tool(
    tools: &[Box<dyn Tool>],
    name: &str,
    args: &Value,
    ctx: &ToolContext,
) -> Result<String> {
    let tool = tools
        .iter()
        .find(|t| t.name() == name)
        .ok_or_else(|| anyhow::anyhow!("알 수 없는 도구: {name}"))?;
    tool.execute(args, ctx)
}

/// 도구 호출을 한 줄로 요약(UI 표시용).
fn arg_summary(name: &str, args: &Value) -> String {
    match name {
        "run_shell" => args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "read_file" | "write_file" | "list_dir" => args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "web_search" => args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "web_fetch" => args
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "read_skill" | "save_skill" => args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn first_line(s: &str) -> String {
    let line = s.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        format!("{}", "완료".dimmed())
    } else {
        line.to_string()
    }
}
