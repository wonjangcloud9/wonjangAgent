//! 클립보드 연동 — 24시간 비서의 자연스러운 입력 채널.
//!
//! 복사한 텍스트/URL을 읽어 번역·요약·저장하거나, 결과를 클립보드에 넣는다.
//! OS별 도구를 사용한다: macOS는 pbpaste/pbcopy, Linux는 wl-paste/xclip/xsel.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// 클립보드 내용을 읽는다.
pub fn read() -> Result<String> {
    match std::env::consts::OS {
        "macos" => run_capture("pbpaste", &[]),
        "linux" => read_linux(),
        other => bail!("이 OS({other})의 클립보드 읽기는 지원하지 않습니다"),
    }
}

/// 클립보드에 텍스트를 쓴다.
pub fn write(text: &str) -> Result<()> {
    let (cmd, args): (&str, &[&str]) = match std::env::consts::OS {
        "macos" => ("pbcopy", &[]),
        "linux" => ("xclip", &["-selection", "clipboard"]),
        other => bail!("이 OS({other})의 클립보드 쓰기는 지원하지 않습니다"),
    };
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .with_context(|| format!("'{cmd}' 실행 실패 — 설치되어 있는지 확인하세요"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    child.wait()?;
    Ok(())
}

/// 리눅스: 여러 클립보드 도구를 순서대로 시도.
fn read_linux() -> Result<String> {
    for (cmd, args) in [
        ("wl-paste", vec!["-n"]),
        ("xclip", vec!["-selection", "clipboard", "-o"]),
        ("xsel", vec!["-b"]),
    ] {
        if let Ok(out) = run_capture(cmd, &args) {
            return Ok(out);
        }
    }
    bail!("클립보드를 읽을 수 없습니다(wl-paste/xclip/xsel 중 하나 필요)")
}

fn run_capture(cmd: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("'{cmd}' 실행 실패"))?;
    if !out.status.success() {
        bail!("'{cmd}' 종료 코드 {:?}", out.status.code());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
