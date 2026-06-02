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

/// 환영 배너. `label`은 백엔드 표기("Claude Code"/"Codex"/"API (...)").
pub fn banner(label: &str) {
    let version = env!("CARGO_PKG_VERSION");
    let keyless = !label.starts_with("API");
    use chrono::Datelike;
    let today = chrono::Local::now().date_naive();
    let weekday = crate::datecalc::weekday_kr(today);

    println!();
    println!(
        "  {}  {}",
        "원장 에이전트".bright_cyan().bold(),
        format!("v{version}").dimmed()
    );
    if keyless {
        println!(
            "  🔑 키 없이 {}에 연결됐어요 — 바로 쓸 수 있어요",
            label.bright_white()
        );
    } else {
        println!("  {} {}", "엔진:".dimmed(), label.bright_white());
    }
    // 성격 묻은 살아있는 첫 인사 + 오늘 날짜.
    println!(
        "  {}  {}",
        crate::soul::greeting().bold(),
        format!("오늘은 {}월 {}일 ({weekday}).", today.month(), today.day()).dimmed()
    );
    println!(
        "  {}",
        "무엇이든 한국어로 시켜보세요.  /help · /성격 · /exit".dimmed()
    );
    println!();
}
