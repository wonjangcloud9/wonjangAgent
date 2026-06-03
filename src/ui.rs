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

/// 내부 도구 이름을 한국 사용자에게 친근한 (이모지, 라벨)로.
fn friendly_tool(name: &str) -> (&'static str, &'static str) {
    match name {
        "run_shell" => ("💻", "명령 실행"),
        "read_file" => ("📄", "파일 읽기"),
        "write_file" => ("✍️", "파일 쓰기"),
        "list_dir" => ("📁", "폴더 살펴보기"),
        "web_search" => ("🔍", "웹 검색"),
        "web_fetch" => ("🌐", "웹페이지 가져오기"),
        "read_skill" | "list_skills" => ("📖", "스킬 펼치기"),
        "save_skill" => ("💾", "스킬 저장"),
        "remember" => ("🧠", "기억하기"),
        "recall" => ("🧠", "기억 떠올리기"),
        "weather_now" => ("🌤️", "날씨 확인"),
        "air_quality" => ("😷", "미세먼지 확인"),
        "subway_arrivals" => ("🚇", "지하철 도착 조회"),
        "exchange_rate" => ("💱", "환율 조회"),
        "coin_price" => ("🪙", "코인 시세 조회"),
        "news_headlines" => ("📰", "뉴스 확인"),
        "lotto_numbers" => ("🎱", "로또 번호 뽑기"),
        "note_search" | "note_read" | "note_list" => ("📒", "노트 찾아보기"),
        "note_append" => ("📒", "노트 기록"),
        "notion_search" | "notion_append" => ("🗂️", "노션 작업"),
        "read_clipboard" | "write_clipboard" => ("📋", "클립보드"),
        "add_reminder" | "list_reminders" | "remove_reminder" => ("⏰", "약속·알림"),
        "add_todo" | "list_todos" | "complete_todo" => ("✅", "할 일"),
        "add_dday" | "list_ddays" => ("📅", "디데이"),
        "add_expense" | "expense_summary" => ("💰", "가계부"),
        "add_habit" | "check_habit" | "list_habits" => ("🔥", "습관"),
        "spawn_subagent" | "spawn_subagents" => ("🤝", "도우미 호출"),
        _ => ("⚙", ""),
    }
}

/// 도구 실행 알림(자주색).
pub fn tool_call(name: &str, summary: &str) {
    let (emoji, label) = friendly_tool(name);
    // 매핑이 있으면 한국어 라벨, 없으면 내부 이름 그대로.
    let title = if label.is_empty() { name } else { label };
    println!(
        "  {} {} {}",
        emoji,
        title.bright_magenta().bold(),
        summary.dimmed()
    );
}

/// 도구 결과 요약(들여쓰기).
pub fn tool_result(summary: &str) {
    println!("    {} {}", "↳".dimmed(), summary.dimmed());
}

/// 환영 배너. `label`은 백엔드 표기("Claude Code"/"Codex"/"API (...)").
/// `ready`는 그 백엔드를 지금 바로 쓸 수 있는지(CLI 백엔드면 바이너리 설치 여부).
pub fn banner(label: &str, ready: bool) {
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
    if keyless && ready {
        println!(
            "  🔑 키 없이 {}에 연결됐어요 — 바로 쓸 수 있어요",
            label.bright_white()
        );
    } else if keyless {
        // 백엔드 CLI가 아직 없을 때: 거짓 약속("연결됐어요") 대신 정직한 안내.
        // 자연어(에이전트)만 그 CLI가 필요하고, 아래 빌트인 기능은 지금도 다 된다.
        println!(
            "  💡 {} 연결 전 — 설치·로그인하면 자연어 명령까지 (아래 기능들은 지금 바로 OK)",
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
    // '수집→자랑' 루프를 첫 화면에 각인: 오늘까지의 미니 잔디 한 줄.
    println!("  {}", habit_strip());
    println!(
        "  {}",
        "무엇이든 한국어로 시켜보세요.  /help · /성격 · /자랑 · /exit".dimmed()
    );
    println!();
}

/// 배너용 미니 잔디 한 줄. 습관이 있으면 가장 긴 연속 + 최근 7일 ▓░,
/// 없으면 빈 잔디로 '수집→자랑' 루프를 유도한다.
fn habit_strip() -> String {
    let store = match crate::habits::HabitStore::load() {
        Ok(s) => s,
        Err(_) => return "오늘부터 한 칸씩 쌓아봐요 🌱".dimmed().to_string(),
    };
    if store.items.is_empty() {
        return format!(
            "{}  {}",
            "░░░░░░░".dimmed(),
            "습관 하나만 등록하면 — 한 달 뒤 자랑 카드가 생겨요 (wonjang 자랑)".dimmed()
        );
    }
    let today = crate::habits::today();
    let best = store
        .items
        .iter()
        .max_by_key(|h| h.streak(today))
        .expect("items non-empty");
    let s = best.streak(today);
    let set = best.date_set();
    let bools: Vec<bool> = (0..7)
        .rev()
        .map(|i| {
            let d = today - chrono::Duration::days(i);
            set.contains(&d.format("%Y-%m-%d").to_string())
        })
        .collect();
    let jandi = crate::card::render_jandi(&bools);
    if s > 0 {
        format!(
            "🔥 {} {}일째   {}",
            best.name.bright_white(),
            s.bright_yellow(),
            jandi.green()
        )
    } else {
        format!("{}  {}", jandi.dimmed(), "오늘 한 칸 채워볼까요?".dimmed())
    }
}

/// 처음 쓰는 사용자에게 한 번만 보여주는 따뜻한 안내.
pub fn onboarding_if_first() {
    let marker = match dirs::data_dir() {
        Some(d) => d.join("wonjang").join(".welcomed"),
        None => return,
    };
    if marker.exists() {
        return;
    }
    println!(
        "  {} 처음 오셨네요! 무엇이든 한국어로 시키면 제가 알아서 해드려요.",
        "👋".bold()
    );
    println!(
        "     {}",
        "예) \"오늘 서울 날씨\"  ·  \"다운로드 폴더 정리해줘\"".dimmed()
    );
    println!(
        "     {} {}",
        "🌱 오늘부터 한 가지만 쌓아봐요:".dimmed(),
        "wonjang 습관 add 운동".bright_cyan()
    );
    println!(
        "     {}",
        "   매일 한 칸씩 채우면, 한 달 뒤 자랑할 카드가 생겨요 →  wonjang 자랑".dimmed()
    );
    println!(
        "     {}",
        "💡 말투 바꾸기: /성격 친구   ·   📋 전체 기능: wonjang 도움".dimmed()
    );
    println!();
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&marker, "1").ok();
}
