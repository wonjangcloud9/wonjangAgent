//! 엔진 — 백엔드(API / Claude Code / Codex)를 추상화한다.
//!
//! 호출부(REPL·단발·프리셋·텔레그램·크론)는 백엔드 종류와 무관하게 `Engine::run`만
//! 호출한다. API 백엔드는 원장의 자체 도구 루프를, CLI 백엔드는 사용자의
//! Claude Code/Codex를 사용한다.

use crate::agent;
use crate::cli_backend::{self, CliKind};
use crate::config::Config;
use crate::llm::{LlmClient, Message};
use crate::tools::{Tool, ToolContext};
use anyhow::{bail, Result};

/// 결정된 백엔드 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Api,
    Claude,
    Codex,
}

/// 설정과 환경으로부터 사용할 백엔드를 결정한다.
pub fn resolve(cfg: &Config) -> Result<Backend> {
    match cfg.backend.trim().to_lowercase().as_str() {
        "api" => Ok(Backend::Api),
        "claude" => Ok(Backend::Claude),
        "codex" => Ok(Backend::Codex),
        "" | "auto" => {
            if !cfg.api_key.is_empty() {
                Ok(Backend::Api)
            } else if command_exists(CliKind::Claude.binary()) {
                Ok(Backend::Claude)
            } else if command_exists(CliKind::Codex.binary()) {
                Ok(Backend::Codex)
            } else {
                bail!(
                    "사용할 백엔드를 찾지 못했습니다.\n  \
                     - API 키를 설정하거나(예: export OPENROUTER_API_KEY=sk-...)\n  \
                     - Claude Code(claude) 또는 Codex(codex) CLI를 설치/로그인하세요."
                )
            }
        }
        other => bail!("알 수 없는 backend 설정: '{other}' (api/claude/codex/auto 중 하나)"),
    }
}

/// 실행 엔진. API 백엔드는 클라이언트와 도구를 보유한다.
pub enum Engine {
    Api {
        client: LlmClient,
        tools: Vec<Box<dyn Tool>>,
    },
    Cli(CliKind),
}

impl Engine {
    /// 한 턴을 실행하고 최종 답변을 반환한다(messages에 기록).
    pub async fn run(
        &self,
        cfg: &Config,
        ctx: &ToolContext,
        messages: &mut Vec<Message>,
    ) -> Result<Option<String>> {
        match self {
            Engine::Api { client, tools } => {
                agent::run_turn(client, cfg, tools, ctx, messages).await
            }
            Engine::Cli(kind) => cli_backend::run(*kind, cfg, ctx, messages).await,
        }
    }

    /// 사람이 읽는 백엔드 이름.
    pub fn label(&self, cfg: &Config) -> String {
        match self {
            Engine::Api { .. } => format!("API ({})", cfg.model),
            Engine::Cli(kind) => kind.label().to_string(),
        }
    }
}

/// 명령이 PATH에 존재하는지 검사한다(실행하지 않음).
pub fn command_exists(cmd: &str) -> bool {
    let path = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return true;
        }
        // Windows 확장자.
        for ext in ["exe", "cmd", "bat"] {
            if candidate.with_extension(ext).is_file() {
                return true;
            }
        }
    }
    false
}
