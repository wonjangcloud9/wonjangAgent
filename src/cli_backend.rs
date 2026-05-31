//! CLI 백엔드 — 사용자의 Claude Code / Codex CLI를 엔진으로 사용.
//!
//! 별도 API 키 없이, 이미 로그인된 `claude`(Claude Code) 또는 `codex`(Codex CLI)에
//! 작업을 위임한다. 원장은 한국어 페르소나·메모리·스킬·프리셋·게이트웨이(텔레그램/
//! 크론) 등 사용자 경험을 담당하고, 실제 추론·로컬 작업은 CLI 엔진이 수행한다.
//!
//! 이 백엔드에서는 원장의 자체 도구 루프 대신 CLI가 자신의 도구로 작업하므로,
//! 원장의 위험 명령 안전장치 대신 CLI 자체의 권한 정책을 따른다. 자동 승인
//! (`-y`/무인 모드)일 때 쓰기 도구까지 허용하고, 아니면 읽기 전용으로 제한한다.

use crate::config::Config;
use crate::llm::Message;
use crate::tools::ToolContext;
use crate::ui;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// 어떤 CLI 백엔드인가.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliKind {
    Claude,
    Codex,
}

impl CliKind {
    pub fn binary(&self) -> &'static str {
        match self {
            CliKind::Claude => "claude",
            CliKind::Codex => "codex",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CliKind::Claude => "Claude Code",
            CliKind::Codex => "Codex",
        }
    }
}

#[derive(Deserialize)]
struct ClaudeResult {
    #[serde(default)]
    result: String,
    #[serde(default)]
    is_error: bool,
}

/// 한 턴을 CLI 백엔드로 실행한다. 최종 답변을 반환하고 messages에 기록한다.
pub async fn run(
    kind: CliKind,
    _cfg: &Config,
    ctx: &ToolContext,
    messages: &mut Vec<Message>,
) -> Result<Option<String>> {
    let system = system_text(messages);
    let prompt = render_prompt(messages);

    let answer = match kind {
        CliKind::Claude => run_claude(&system, &prompt, ctx.auto_approve).await?,
        CliKind::Codex => run_codex(&prompt, &system).await?,
    };

    messages.push(Message {
        role: "assistant".into(),
        content: Some(answer.clone()),
        tool_calls: None,
        tool_call_id: None,
    });
    Ok(Some(answer))
}

/// 시스템 메시지(있으면)를 추출.
fn system_text(messages: &[Message]) -> String {
    messages
        .iter()
        .find(|m| m.role == "system")
        .and_then(|m| m.content.clone())
        .unwrap_or_default()
}

/// 대화(시스템 제외)를 하나의 프롬프트 텍스트로 렌더링한다.
///
/// CLI는 매 호출이 독립적이므로, 이전 맥락을 텍스트로 함께 넘겨 연속성을 준다.
fn render_prompt(messages: &[Message]) -> String {
    let convo: Vec<&Message> = messages.iter().filter(|m| m.role != "system").collect();
    if convo.len() <= 1 {
        // 첫 턴: 사용자 메시지만.
        return convo
            .last()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();
    }
    let mut out = String::from("이전 대화:\n");
    for m in &convo[..convo.len() - 1] {
        let who = if m.role == "assistant" {
            "원장"
        } else {
            "사용자"
        };
        if let Some(c) = &m.content {
            out.push_str(&format!("{who}: {c}\n"));
        }
    }
    if let Some(last) = convo.last().and_then(|m| m.content.clone()) {
        out.push_str(&format!("\n현재 요청: {last}"));
    }
    out
}

/// Claude Code(claude -p)에 위임한다.
async fn run_claude(system: &str, prompt: &str, auto_approve: bool) -> Result<String> {
    // 자동 승인이면 쓰기 도구까지, 아니면 읽기 전용으로 제한.
    let allowed = if auto_approve {
        "Bash Read Write Edit Glob Grep WebSearch WebFetch"
    } else {
        "Read Glob Grep WebSearch WebFetch"
    };

    let mut cmd = Command::new("claude");
    cmd.args([
        "--print",
        "--output-format",
        "json",
        "--append-system-prompt",
        system,
        "--allowedTools",
        allowed,
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());

    ui::tool_result(&format!("Claude Code에 위임 중… (허용 도구: {allowed})"));

    let mut child = cmd.spawn().context(
        "claude 실행 실패 — Claude Code가 설치/로그인되어 있는지 확인하세요(claude --version)",
    )?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).await?;
        stdin.shutdown().await.ok();
    }
    let output = child.wait_with_output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        bail!(
            "claude 응답이 비어 있습니다(종료 코드 {:?})",
            output.status.code()
        );
    }
    let parsed: ClaudeResult =
        serde_json::from_str(stdout.trim()).context("claude JSON 응답 파싱 실패")?;
    if parsed.is_error {
        bail!("Claude Code 오류: {}", parsed.result);
    }
    Ok(parsed.result)
}

/// Codex(codex exec)에 위임한다(실험적 — stdout 텍스트를 그대로 반환).
async fn run_codex(prompt: &str, system: &str) -> Result<String> {
    let full = format!("{system}\n\n{prompt}");
    let output = Command::new("codex")
        .arg("exec")
        .arg(&full)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .context("codex 실행 실패 — Codex CLI가 설치/로그인되어 있는지 확인하세요")?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        bail!(
            "codex 응답이 비어 있습니다(종료 코드 {:?})",
            output.status.code()
        );
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str) -> Message {
        Message {
            role: role.into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    #[test]
    fn render_first_turn_is_user_only() {
        let m = vec![msg("system", "페르소나"), msg("user", "안녕")];
        assert_eq!(render_prompt(&m), "안녕");
    }

    #[test]
    fn render_multi_turn_includes_history() {
        let m = vec![
            msg("system", "페르소나"),
            msg("user", "첫 질문"),
            msg("assistant", "첫 답변"),
            msg("user", "두번째 질문"),
        ];
        let p = render_prompt(&m);
        assert!(p.contains("사용자: 첫 질문"));
        assert!(p.contains("원장: 첫 답변"));
        assert!(p.contains("현재 요청: 두번째 질문"));
    }

    #[test]
    fn system_text_extracted() {
        let m = vec![msg("system", "페르소나"), msg("user", "x")];
        assert_eq!(system_text(&m), "페르소나");
    }
}
