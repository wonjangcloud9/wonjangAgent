//! 쉘 명령 실행 도구.
//!
//! 로컬 환경을 직접 다루는 핵심 도구. 안전을 위해 기본적으로 실행 전
//! 사용자 승인을 요구한다(`--yes`로 자동 승인 가능).

use super::{Tool, ToolContext, ToolSpec};
use anyhow::Result;
use owo_colors::OwoColorize;
use serde_json::Value;
use std::io::{self, Write};
use std::process::Command;

pub struct ShellTool;

impl Tool for ShellTool {
    fn name(&self) -> &'static str {
        "run_shell"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "run_shell",
            description: "로컬 셸(zsh/bash)에서 명령을 실행하고 표준 출력/오류를 반환합니다. \
                파일 조작, 빌드, git 등 시스템 작업에 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "실행할 셸 명령 한 줄. 예: 'ls -la', 'git status'"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    fn execute(&self, args: &Value, ctx: &ToolContext) -> Result<String> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("'command' 인자가 필요합니다"))?;

        if !ctx.auto_approve && !confirm(command)? {
            return Ok("사용자가 명령 실행을 거부했습니다.".to_string());
        }

        let output = Command::new("zsh").arg("-lc").arg(command).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);

        let mut result = format!("종료 코드: {code}\n");
        if !stdout.trim().is_empty() {
            result.push_str(&format!("--- stdout ---\n{}\n", truncate(&stdout)));
        }
        if !stderr.trim().is_empty() {
            result.push_str(&format!("--- stderr ---\n{}\n", truncate(&stderr)));
        }
        if stdout.trim().is_empty() && stderr.trim().is_empty() {
            result.push_str("(출력 없음)\n");
        }
        Ok(result)
    }
}

/// 실행 전 사용자에게 y/n 확인을 받는다.
fn confirm(command: &str) -> Result<bool> {
    print!(
        "  {} {}\n  {} ",
        "▶ 셸 실행:".bright_yellow().bold(),
        command.bright_white(),
        "진행할까요? [y/N]".yellow()
    );
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let ans = input.trim().to_lowercase();
    Ok(ans == "y" || ans == "yes" || ans == "예")
}

/// 너무 긴 출력을 잘라 컨텍스트 폭주를 막는다.
fn truncate(s: &str) -> String {
    const MAX: usize = 8000;
    if s.len() <= MAX {
        s.to_string()
    } else {
        format!(
            "{}\n… (출력이 길어 {}자에서 잘림)",
            &s[..MAX],
            MAX
        )
    }
}
