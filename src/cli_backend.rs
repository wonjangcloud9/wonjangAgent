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

    /// 이 백엔드 CLI가 PATH에 설치돼 있는지(프로세스 실행 없이 PATH 디렉터리만 확인).
    /// 배너가 "연결됐어요"를 거짓으로 약속하지 않도록 가용성 판정에 쓴다.
    pub fn is_available(&self) -> bool {
        let bin = self.binary();
        match std::env::var_os("PATH") {
            Some(paths) => std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()),
            None => false,
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
        CliKind::Codex => run_codex(&prompt, &system, ctx.auto_approve).await?,
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

/// 원장 도구(기억·스킬·알림·날씨 등)를 claude에 물려주는 MCP 설정(인라인 JSON).
/// 실행 파일 경로를 못 얻으면 None — MCP 없이도 위임 자체는 동작해야 한다.
fn wonjang_mcp_config() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(
        serde_json::json!({
            "mcpServers": { "wonjang": { "command": exe.to_str()?, "args": ["mcp-serve"] } }
        })
        .to_string(),
    )
}

/// Claude Code(claude -p)에 위임한다.
async fn run_claude(system: &str, prompt: &str, auto_approve: bool) -> Result<String> {
    // 자동 승인이면 쓰기 도구까지, 아니면 읽기 전용으로 제한.
    // 원장 비서 도구는 MCP(mcp__wonjang)로 두 모드 모두 허용 — 셸·파일은 안 노출되므로
    // 읽기전용 정책을 우회하지 않는다(mcp_server::served_tools 참고).
    let base = if auto_approve {
        "Bash Read Write Edit Glob Grep WebSearch WebFetch"
    } else {
        "Read Glob Grep WebSearch WebFetch"
    };
    let mcp_config = wonjang_mcp_config();
    let allowed = if mcp_config.is_some() {
        format!("{base} mcp__wonjang")
    } else {
        base.to_string()
    };
    // 위임받은 모델이 자기(클라이언트) 메모리 기능으로 새면 원장 메모리가 안 쌓여
    // 성장 루프가 끊긴다(라이브 검증으로 실측된 함정) → 정확한 MCP 도구명을 못박는다.
    let system = if mcp_config.is_some() {
        format!(
            "{system}\n\n도구 규칙(중요): 기억은 반드시 mcp__wonjang__remember 도구로 저장하고 \
             mcp__wonjang__recall로 조회하세요. 자체 메모리 기능·파일 기록 등 다른 방식으로 \
             기억을 남기지 마세요. 알림·할일·디데이·가계부·습관·스킬·날씨·환율 등도 \
             mcp__wonjang__ 도구가 있으면 그것을 우선 사용하세요."
        )
    } else {
        system.to_string()
    };

    let mut cmd = Command::new("claude");
    cmd.args([
        "--print",
        "--output-format",
        "json",
        "--append-system-prompt",
        &system,
        "--allowedTools",
        &allowed,
    ]);
    if let Some(cfg) = &mcp_config {
        cmd.args(["--mcp-config", cfg]);
    }
    cmd.stdin(Stdio::piped())
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
        bail!("{}", friendly_backend_error(&parsed.result));
    }
    Ok(parsed.result)
}

/// Claude Code 에러를 wonjang 사용자용 메시지로. 로그인 안 됨이면 `/login`(Claude Code
/// 개념)을 그대로 노출하는 대신, AI 기능엔 로그인이 필요하고 오프라인 기능은 바로
/// 쓸 수 있음을 안내한다(신규 사용자가 처음 AI를 시도하는 첫인상 순간이라 중요).
fn friendly_backend_error(raw: &str) -> String {
    let r = raw.trim();
    if r.contains("Not logged in") || r.contains("/login") {
        "AI 대화 기능은 Claude Code 로그인이 필요해요.\n     \
         • 설치·로그인: claude (claude.com/claude-code) 설치 후 로그인\n     \
         • 로그인 없이 바로 쓰는 기능: wonjang 도움 (날씨·계산기·디데이 등 대부분 OK)"
            .to_string()
    } else {
        format!("Claude Code 오류: {r}")
    }
}

/// Codex(codex exec)에 위임한다(실험적 — stdout 텍스트를 그대로 반환).
async fn run_codex(prompt: &str, system: &str, auto_approve: bool) -> Result<String> {
    let full = format!("{system}\n\n{prompt}");
    // 자동 승인이 아니면 read-only 샌드박스로 제한(Claude 백엔드의 읽기전용 도구셋과 동등).
    // 이게 없으면 codex exec가 wonjang의 읽기전용 의도를 무시하고 파일 쓰기·명령을 실행한다(보안).
    let sandbox = if auto_approve {
        "workspace-write"
    } else {
        "read-only"
    };
    let output = Command::new("codex")
        .arg("exec")
        .arg("--sandbox")
        .arg(sandbox)
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
    fn login_error_is_friendly_and_points_offline() {
        // 로그인 안 됨 → '/login'(Claude Code 개념) 노출 대신 친절 안내 + 오프라인 유도.
        let f = friendly_backend_error("Not logged in · Please run /login");
        assert!(f.contains("로그인이 필요"));
        assert!(f.contains("wonjang 도움")); // 오프라인 기능으로 유도
        assert!(!f.contains("Claude Code 오류")); // 날것 에러 접두사 안 씀
                                                  // 다른 에러는 종전대로 노출(진단 가능).
        let other = friendly_backend_error("rate limit exceeded");
        assert!(other.contains("Claude Code 오류"));
        assert!(other.contains("rate limit"));
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
