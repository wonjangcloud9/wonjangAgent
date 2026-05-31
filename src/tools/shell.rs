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

        // 안전장치: 위험 명령은 무인 모드에서 차단, 대화형에서 강한 경고.
        match crate::safety::classify_danger(command) {
            Some(reason) => {
                if ctx.auto_approve {
                    if !ctx.allow_dangerous {
                        return Ok(format!(
                            "⛔ 위험 명령 차단({reason}): {command}\n\
                             자동 승인 모드에서는 실행하지 않습니다. \
                             정말 필요하면 사람이 직접 실행하거나 --allow-dangerous 로 명시 허용하세요."
                        ));
                    }
                    println!(
                        "  {} {} {}",
                        "⚠️ 위험 명령 자동 실행".bright_red().bold(),
                        format!("({reason})").red(),
                        command.bright_white()
                    );
                } else if !confirm_dangerous(command, reason)? {
                    return Ok("사용자가 위험 명령 실행을 거부했습니다.".to_string());
                }
            }
            None => {
                if !ctx.auto_approve && !confirm(command)? {
                    return Ok("사용자가 명령 실행을 거부했습니다.".to_string());
                }
            }
        }

        let (shell, flag) = pick_shell();
        let output = Command::new(&shell).arg(flag).arg(command).output()?;

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

/// 위험 명령에 대한 강한 경고 + 확인.
fn confirm_dangerous(command: &str, reason: &str) -> Result<bool> {
    println!();
    println!(
        "  {} {}",
        "⚠️  위험 명령 감지".bright_red().bold(),
        format!("— {reason}").red()
    );
    print!(
        "  {} {}\n  {} ",
        "▶".bright_red().bold(),
        command.bright_white().bold(),
        "정말 실행할까요? 위험을 이해했다면 [y] 입력:".bright_red()
    );
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let ans = input.trim().to_lowercase();
    Ok(ans == "y" || ans == "yes" || ans == "예")
}

/// 실행에 쓸 셸과 플래그를 이식성 있게 고른다.
///
/// 사용자의 로그인 셸($SHELL)을 우선 사용해 PATH/별칭을 그대로 활용하고,
/// 없으면 흔한 셸로 폴백한다. zsh/bash는 로그인 모드(-lc), sh 폴백은 -c.
fn pick_shell() -> (String, &'static str) {
    if let Ok(sh) = std::env::var("SHELL") {
        if !sh.is_empty() && std::path::Path::new(&sh).exists() {
            return (sh, "-lc");
        }
    }
    for cand in ["/bin/zsh", "/bin/bash", "/usr/bin/bash"] {
        if std::path::Path::new(cand).exists() {
            return (cand.to_string(), "-lc");
        }
    }
    ("/bin/sh".to_string(), "-c")
}

/// 너무 긴 출력을 잘라 컨텍스트 폭주를 막는다.
fn truncate(s: &str) -> String {
    const MAX: usize = 8000;
    if s.len() <= MAX {
        s.to_string()
    } else {
        format!("{}\n… (출력이 길어 {}자에서 잘림)", &s[..MAX], MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(auto: bool, danger: bool) -> ToolContext {
        ToolContext {
            auto_approve: auto,
            allow_dangerous: danger,
        }
    }

    #[test]
    fn blocks_dangerous_in_auto_mode() {
        let out = ShellTool
            .execute(
                &json!({ "command": "rm -rf /tmp/no_such_xyz" }),
                &ctx(true, false),
            )
            .unwrap();
        assert!(out.contains("차단"), "차단되어야 함: {out}");
    }

    #[test]
    fn runs_safe_in_auto_mode() {
        let out = ShellTool
            .execute(&json!({ "command": "echo 안녕하세요" }), &ctx(true, false))
            .unwrap();
        assert!(out.contains("안녕하세요"), "출력: {out}");
    }

    #[test]
    fn allows_dangerous_when_opted_in() {
        // killall(위험 분류)이지만 없는 프로세스라 무해 — 차단되지 않고 실행됨.
        let out = ShellTool
            .execute(
                &json!({ "command": "killall definitely_no_such_proc_xyz" }),
                &ctx(true, true),
            )
            .unwrap();
        assert!(!out.contains("차단"), "허용 시 실행되어야 함: {out}");
        assert!(out.contains("종료 코드"));
    }
}
