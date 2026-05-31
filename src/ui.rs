//! 한국어 우선 터미널 UI 유틸리티.
//!
//! 모든 사용자 대면 문자열은 이 모듈을 거쳐 출력해 톤과 색을 일관되게 유지한다.

use owo_colors::OwoColorize;

/// 에이전트 이름(브랜딩).
#[allow(dead_code)]
pub const AGENT_NAME: &str = "원장";

/// 에이전트가 말할 때 쓰는 프롬프트 라벨.
pub fn agent_label() -> String {
    format!("{}", "원장".bright_cyan().bold())
}

/// 사용자 입력 프롬프트.
pub fn prompt() -> String {
    format!("{} ", "당신 ▸".bright_green().bold())
}

/// 정보성 메시지(회색).
pub fn info(msg: &str) {
    println!("{}", msg.dimmed());
}

/// 강조 메시지(노란색).
pub fn note(msg: &str) {
    println!("{}", msg.yellow());
}

/// 오류 메시지(빨간색, stderr).
pub fn error(msg: &str) {
    eprintln!("{} {}", "오류:".bright_red().bold(), msg.red());
}

/// 도구 실행 알림(자주색).
pub fn tool_call(name: &str, summary: &str) {
    println!(
        "  {} {} {}",
        "⚙".bright_magenta(),
        name.bright_magenta().bold(),
        summary.dimmed()
    );
}

/// 도구 결과 요약(들여쓰기).
pub fn tool_result(summary: &str) {
    println!("    {} {}", "↳".dimmed(), summary.dimmed());
}

/// 환영 배너.
pub fn banner(model: &str) {
    println!();
    println!(
        "  {}  {}",
        "원장 에이전트".bright_cyan().bold(),
        "v0.1.0".dimmed()
    );
    println!(
        "  {}",
        "로컬 환경을 다루는 한국어 우선 AI 에이전트".dimmed()
    );
    println!("  {} {}", "모델:".dimmed(), model.bright_white());
    println!(
        "  {}",
        "도움말은 /help, 종료는 /exit 또는 Ctrl-D".dimmed()
    );
    println!();
}
