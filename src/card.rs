//! 자랑 카드 — 로컬에 쌓인 데이터를 '카톡 코드블록에서도 안 깨지는' 한 장 ANSI 회고 카드로.
//!
//! 핵심: 한글·전각·이모지는 터미널에서 2칸을 차지한다. 박스(╭─╮)가 정렬되려면
//! 모든 줄의 '표시 폭'을 정확히 같게 맞춰야 한다. `disp_width`가 그 기준이다.
//! PNG는 만들지 않는다(한글 TTF가 musl 정적 바이너리를 비대화 — 빌드정책).

use unicode_width::UnicodeWidthChar;

/// 한 글자의 터미널 표시 폭. 결합문자·변이선택자=0, 한글·전각·흔한 이모지=2.
fn char_width(c: char) -> usize {
    match c {
        '\u{FE0F}' | '\u{FE0E}' | '\u{200D}' => 0, // 변이 선택자·ZWJ
        _ => {
            let cp = c as u32;
            // 흔한 이모지는 macOS 터미널·카톡에서 2칸으로 렌더된다.
            let emoji = (0x1F300..=0x1FAFF).contains(&cp)
                || (0x2600..=0x27BF).contains(&cp) // ✍ ✅ 등 기타기호·딩벳
                || cp == 0x2B50
                || cp == 0x2B55;
            if emoji {
                2
            } else {
                UnicodeWidthChar::width(c).unwrap_or(0)
            }
        }
    }
}

/// 문자열의 터미널 표시 폭(카톡·터미널 정렬 기준).
pub fn disp_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// 표시 폭이 `max` 이하가 되도록 글자 경계에서 자른다.
fn truncate_width(s: &str, max: usize) -> String {
    let mut w = 0;
    let mut out = String::new();
    for c in s.chars() {
        let cw = char_width(c);
        if w + cw > max {
            break;
        }
        w += cw;
        out.push(c);
    }
    out
}

/// 내용 줄: `│ {body}{...공백} │` — 안쪽 폭 `inner`에 정확히 맞춘다.
fn content(body: &str, inner: usize) -> String {
    let s = format!(" {body}");
    let s = if disp_width(&s) > inner {
        truncate_width(&s, inner)
    } else {
        s
    };
    let pad = inner.saturating_sub(disp_width(&s));
    format!("│{}{}│", s, " ".repeat(pad))
}

/// 라벨·값 정렬 줄: 라벨을 `label_col` 표시폭으로 패딩한 뒤 값.
fn row(label: &str, value: &str, inner: usize, label_col: usize) -> String {
    let pad = label_col.saturating_sub(disp_width(label));
    content(&format!("{label}{}{value}", " ".repeat(pad)), inner)
}

/// 가로줄(테두리/구분선). `title`을 가운데 두고 ─로 채운다.
fn rule(left: char, right: char, title: &str, inner: usize) -> String {
    let label = if title.is_empty() {
        String::new()
    } else {
        truncate_width(&format!(" {title} "), inner)
    };
    let lw = disp_width(&label);
    let dash = inner.saturating_sub(lw);
    let l = dash / 2;
    let r = dash - l;
    format!("{left}{}{}{}{right}", "─".repeat(l), label, "─".repeat(r))
}

/// 28일 잔디(오래된→최신)를 7개씩 묶어 렌더. ▓=완료, ░=미완.
pub fn render_jandi(days: &[bool]) -> String {
    let mut out = String::new();
    for (i, chunk) in days.chunks(7).enumerate() {
        if i > 0 {
            out.push(' ');
        }
        for &d in chunk {
            out.push(if d { '▓' } else { '░' });
        }
    }
    out
}

/// 카드에 들어갈 데이터(읽기 전용으로 모은 로컬 통계).
pub struct CardData {
    pub title: String,
    pub streak: Option<(String, i64)>,
    pub jandi: Vec<bool>,
    pub focus_label: String,
    pub expense_label: String,
    pub dday: Option<String>,
    pub journal_count: usize,
    pub comment: String,
    pub footer: String,
}

/// 카드를 줄 단위 플레인 문자열로 렌더한다(색 없음 — 출력 측에서 입힘).
/// 모든 줄의 `disp_width`가 동일(=width)함이 박스 정렬의 핵심 불변식.
pub fn render_card(d: &CardData, width: usize) -> Vec<String> {
    let w = width.clamp(30, 80);
    let inner = w - 2;
    let col = 16; // 라벨 정렬 열(표시폭)
    let mut lines = Vec::new();
    lines.push(rule('╭', '╮', &format!("원장 카드 · {}", d.title), inner));
    if let Some((name, s)) = &d.streak {
        lines.push(row("🔥 가장 긴 연속", &format!("{name} {s}일"), inner, col));
    }
    if !d.jandi.is_empty() {
        // 잔디는 폭이 커서(최대 31칸) 라벨 없이 한 줄 통째로 — 좁은 카톡 폭(34)에도 들어가게.
        lines.push(content(&render_jandi(&d.jandi), inner));
    }
    lines.push(row("🍅 이번 달 집중", &d.focus_label, inner, col));
    lines.push(row("💰 이번 달 지출", &d.expense_label, inner, col));
    if let Some(dday) = &d.dday {
        lines.push(row("📅 다가오는 날", dday, inner, col));
    }
    lines.push(row(
        "✍️ 이번 달 일기",
        &format!("{}번", d.journal_count),
        inner,
        col,
    ));
    lines.push(rule('├', '┤', "", inner));
    lines.push(content(&format!("💬 {}", d.comment), inner));
    lines.push(rule('╰', '╯', &d.footer, inner));
    lines
}

/// 주간 카드 데이터(이번 주 + 지난주 대비).
pub struct WeeklyCardData {
    pub title: String,
    pub streak: Option<(String, i64)>,
    pub jandi7: Vec<bool>,
    pub focus_value: String,   // 예: "12시간 30분  ▲2시간"
    pub expense_value: String, // 예: "240,000원  ▼50,000"
    pub comment: String,
    pub footer: String,
}

/// 주간 카드를 줄 단위로 렌더(월간과 같은 박스 시스템·전각폭 불변식).
pub fn render_weekly_card(d: &WeeklyCardData, width: usize) -> Vec<String> {
    let w = width.clamp(30, 80);
    let inner = w - 2;
    let col = 16;
    let mut lines = Vec::new();
    lines.push(rule('╭', '╮', &format!("원장 주간 · {}", d.title), inner));
    if let Some((name, s)) = &d.streak {
        lines.push(row("🔥 가장 긴 연속", &format!("{name} {s}일"), inner, col));
    }
    if !d.jandi7.is_empty() {
        lines.push(content(&render_jandi(&d.jandi7), inner));
    }
    lines.push(row("🍅 이번 주 집중", &d.focus_value, inner, col));
    lines.push(row("💰 이번 주 지출", &d.expense_value, inner, col));
    lines.push(rule('├', '┤', "", inner));
    lines.push(content(&format!("💬 {}", d.comment), inner));
    lines.push(rule('╰', '╯', &d.footer, inner));
    lines
}

/// 증감 화살표(지난주 대비). 순수.
pub fn delta_arrow(delta: i64) -> &'static str {
    if delta > 0 {
        "▲"
    } else if delta < 0 {
        "▼"
    } else {
        "·"
    }
}

/// 주간 코멘트 — 집중 증감 기준 페르소나 말투.
pub fn weekly_comment(persona_key: &str, focus_delta: i64) -> String {
    if focus_delta > 0 {
        match persona_key {
            "친구" => "이번 주 집중 늘었네! 가보자 💪".to_string(),
            "집사" => "지난주보다 더 정진하셨습니다, 주인님.".to_string(),
            "선배" => "지난주보다 나아졌네. 좋아.".to_string(),
            "발랄" => "집중 업업! 이번 주 멋졌어✨".to_string(),
            _ => "지난주보다 집중이 늘었어요. 좋아요!".to_string(),
        }
    } else if focus_delta < 0 {
        match persona_key {
            "친구" => "이번 주는 좀 쉬어갔네. 다음 주 가자!".to_string(),
            "발랄" => "이번 주는 충전! 다음 주 다시 달리자✨".to_string(),
            _ => "이번 주는 잠시 쉬어갔네요. 다음 주 다시!".to_string(),
        }
    } else {
        "꾸준히 가고 있어요.".to_string()
    }
}

/// 페르소나별 카드 코멘트(자랑 말투).
pub fn card_comment(persona_key: &str, streak: i64, habit: &str) -> String {
    if streak >= 2 {
        match persona_key {
            "친구" => format!("{habit} {streak}일 연속, 너 좀 멋진데? 🙌"),
            "집사" => format!("{habit} {streak}일 연속이십니다. 빈틈없으셨어요, 주인님."),
            "선배" => format!("{streak}일 연속. 꾸준한 거 인정."),
            "발랄" => format!("우와 {habit} {streak}일 연속이라니!! 최고✨"),
            _ => format!("{habit} {streak}일 연속, 잘 쌓고 있어요."),
        }
    } else {
        match persona_key {
            "친구" => "이번 달도 한 칸씩 채워보자 💪".to_string(),
            "발랄" => "오늘 한 칸 콕! 채우러 가자✨".to_string(),
            _ => "오늘 한 칸부터 채워봐요.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hangul_and_ascii_widths() {
        assert_eq!(disp_width("운동"), 4);
        assert_eq!(disp_width("abc"), 3);
        assert_eq!(disp_width("a가b"), 4); // 1+2+1
        assert_eq!(disp_width("▓░▓"), 3); // 잔디 블록은 1칸
    }

    #[test]
    fn every_line_has_exact_display_width() {
        // 전각/반각/이모지 혼합 카드의 모든 줄이 정확히 같은 표시폭이어야 박스가 안 깨진다.
        let d = CardData {
            title: "2026년 6월".into(),
            streak: Some(("운동".into(), 13)),
            jandi: (0..28).map(|i| i % 3 != 0).collect(),
            focus_label: "42시간 30분".into(),
            expense_label: "1,240,000원".into(),
            dday: Some("토익 D-3".into()),
            journal_count: 9,
            comment: "운동 13일 연속, 너 좀 멋진데? 🙌".into(),
            footer: "wonjang · v1.9.0".into(),
        };
        for width in [34usize, 40, 46, 52] {
            let lines = render_card(&d, width);
            for (i, line) in lines.iter().enumerate() {
                assert_eq!(
                    disp_width(line),
                    width,
                    "width={width} 줄{i} 폭 불일치: {line:?} (={}칸)",
                    disp_width(line)
                );
            }
        }
    }

    #[test]
    fn weekly_card_every_line_exact_width() {
        let d = WeeklyCardData {
            title: "6/1~6/7".into(),
            streak: Some(("운동".into(), 13)),
            jandi7: vec![true, true, false, true, true, true, false],
            focus_value: "12시간 30분  ▲2시간".into(),
            expense_value: "240,000원  ▼50,000".into(),
            comment: "이번 주 집중 늘었네! 가보자 💪".into(),
            footer: "wonjang · v1.13.0".into(),
        };
        for width in [34usize, 40, 46, 52] {
            for line in render_weekly_card(&d, width) {
                assert_eq!(
                    disp_width(&line),
                    width,
                    "주간카드 폭 불일치(w={width}): {line:?}"
                );
            }
        }
    }

    #[test]
    fn delta_arrow_directions() {
        assert_eq!(delta_arrow(120), "▲");
        assert_eq!(delta_arrow(-30), "▼");
        assert_eq!(delta_arrow(0), "·");
        assert!(weekly_comment("친구", 60).contains("늘었"));
        assert!(weekly_comment("기본", -60).contains("쉬어"));
    }

    #[test]
    fn jandi_pattern_exact() {
        // 7일 중 4일 완료 → 정확한 ▓/░ 패턴.
        let days = vec![true, true, false, true, true, true, false];
        assert_eq!(render_jandi(&days), "▓▓░▓▓▓░");
        // 14일 → 7개씩 공백으로 묶임.
        let two = vec![true; 14];
        assert_eq!(render_jandi(&two), "▓▓▓▓▓▓▓ ▓▓▓▓▓▓▓");
    }

    #[test]
    fn comment_varies_by_persona() {
        assert!(card_comment("친구", 13, "운동").contains("멋진데"));
        assert!(card_comment("집사", 5, "독서").contains("주인님"));
        // streak<2면 격려.
        assert!(card_comment("기본", 0, "운동").contains("채워"));
    }

    #[test]
    fn truncate_width_respects_boundary() {
        assert_eq!(truncate_width("운동하기", 4), "운동"); // 2+2
        assert_eq!(truncate_width("운동하기", 5), "운동"); // 한 글자(2칸) 더는 못 들어감
        assert_eq!(truncate_width("abcde", 3), "abc");
    }
}
