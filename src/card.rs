//! 자랑 카드 — 로컬에 쌓인 데이터를 '카톡 코드블록에서도 안 깨지는' 한 장 ANSI 회고 카드로.
//!
//! 핵심: 한글·전각·이모지는 터미널에서 2칸을 차지한다. 박스(╭─╮)가 정렬되려면
//! 모든 줄의 '표시 폭'을 정확히 같게 맞춰야 한다. `disp_width`가 그 기준이다.
//! PNG는 만들지 않는다(한글 TTF가 musl 정적 바이너리를 비대화 — 빌드정책).

use unicode_width::UnicodeWidthChar;

/// 공유 카드 하단 테두리에 박는 전염성 풋터. 카드를 받은 사람에게 '무엇인지·어떻게 받는지'를
/// 알려주는 유일한 성장 고리 표면이라, 내부 버전(설치 불가) 대신 npm 설치명을 노출한다.
/// 표시폭 27(테두리 패딩 포함 29) — 카톡 폭 34(inner 32) 이하에서도 안 잘린다.
pub const SHARE_FOOTER: &str = "나도 만들기 → wonjang-agent";

/// base 한 글자의 폭(unicode-width 기준). 결합문자·변이선택자은 0.
fn base_width(c: char) -> usize {
    match c {
        '\u{FE0F}' | '\u{FE0E}' | '\u{200D}' => 0,
        _ => UnicodeWidthChar::width(c).unwrap_or(0),
    }
}

/// base 글자 뒤에 따라오는 변이선택자를 반영한 폭.
/// 다음 글자가 U+FE0F(이모지 표현)면 2칸, U+FE0E(텍스트 표현)면 1칸, 그 외엔 base 폭.
/// 이렇게 해야 `★♥❤☎` 등 텍스트표현 기호(변이선택자 없음)를 1칸으로, `✍️` 등 이모지표현을 2칸으로 본다.
fn paired_width(c: char, next: Option<char>) -> usize {
    match c {
        '\u{FE0F}' | '\u{FE0E}' | '\u{200D}' => 0,
        _ => match next {
            Some('\u{FE0F}') => 2,
            Some('\u{FE0E}') => 1,
            _ => base_width(c),
        },
    }
}

/// 문자열을 '표시 클러스터' 단위로 나눈다 — (클러스터 문자열, 표시폭).
/// 클러스터 = base 글자 + 뒤따르는 변이선택자(FE0F/FE0E) + ZWJ로 이어진 후속 이모지들.
/// ZWJ 이모지(🏃‍♂️·👨‍💻·👨‍👩‍👧 등)는 한 글자(폭=base)로 친다(wcwidth와 일치).
fn clusters(s: &str) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        let width = paired_width(c, it.peek().copied());
        let mut cl = String::new();
        cl.push(c);
        // 뒤따르는 변이선택자를 같은 클러스터로 흡수.
        while matches!(it.peek(), Some('\u{FE0F}') | Some('\u{FE0E}')) {
            cl.push(it.next().unwrap());
        }
        // ZWJ로 이어진 후속(ZWJ + base + 변이선택자)도 같은 클러스터(폭은 0 추가).
        while it.peek() == Some(&'\u{200D}') {
            cl.push(it.next().unwrap()); // ZWJ
            if let Some(j) = it.next() {
                cl.push(j);
                while matches!(it.peek(), Some('\u{FE0F}') | Some('\u{FE0E}')) {
                    cl.push(it.next().unwrap());
                }
            }
        }
        out.push((cl, width));
    }
    out
}

/// 문자열의 터미널 표시 폭(카톡·터미널 정렬 기준). 변이선택자·ZWJ 이모지 인식.
pub fn disp_width(s: &str) -> usize {
    clusters(s).iter().map(|(_, w)| w).sum()
}

/// 표시 폭이 `max` 이하가 되도록 클러스터 경계에서 자른다(ZWJ 이모지를 중간에 안 쪼갬).
fn truncate_width(s: &str, max: usize) -> String {
    let mut w = 0;
    let mut out = String::new();
    for (cl, cw) in clusters(s) {
        if w + cw > max {
            break;
        }
        out.push_str(&cl);
        w += cw;
    }
    out
}

/// 가로 막대(값/최댓값 비율을 `█`로). 항상 정확히 `width` 표시칸(빈칸은 공백).
/// 엑셀 그룹 집계의 막대그래프 등 한눈 비교에 쓴다. max≤0이면 빈 칸.
pub fn hbar(value: f64, max: f64, width: usize) -> String {
    if max <= 0.0 || width == 0 {
        return " ".repeat(width);
    }
    let ratio = (value / max).clamp(0.0, 1.0);
    let filled = ((ratio * width as f64).round() as usize).min(width);
    format!("{}{}", "█".repeat(filled), " ".repeat(width - filled))
}

/// 표시폭 `width`에 맞춰 자르고(넘으면), 모자라면 오른쪽을 공백으로 채운다.
/// 한글·전각이 섞인 표의 칸 정렬에 쓴다(엑셀 그룹 집계 등).
pub fn truncate_pad(s: &str, width: usize) -> String {
    let t = truncate_width(s, width);
    let w = disp_width(&t);
    if w < width {
        format!("{t}{}", " ".repeat(width - w))
    } else {
        t
    }
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

/// 잔디 줄이 `inner`(선행 공백 1칸 포함)를 넘으면 최신 날짜(뒤)를 보존하며 앞에서 줄인다.
/// 잔디는 ▓░·공백 모두 1칸이라 글자 수 = 표시폭.
fn fit_jandi_tail(jandi: &str, inner: usize) -> String {
    let budget = inner.saturating_sub(1); // content가 앞에 공백 1칸을 더하므로.
    let n = jandi.chars().count();
    if n <= budget {
        return jandi.to_string();
    }
    jandi.chars().skip(n - budget).collect()
}

/// 연속 일수의 성취 등급 뱃지(카드에 표시 — 공유 시 성취 수준을 한눈에=사회적 화폐).
/// 앞에 공백 1칸을 포함해 돌려준다(달성 전엔 빈 문자열).
fn streak_badge(s: i64) -> &'static str {
    match s {
        s if s >= 365 => " 👑",
        s if s >= 100 => " 🏆",
        s if s >= 30 => " 🎊",
        s if s >= 7 => " 🎉",
        _ => "",
    }
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
        let value = format!("{name} {s}일{}", streak_badge(*s));
        lines.push(row("🔥 가장 긴 연속", &value, inner, col));
    }
    if !d.jandi.is_empty() {
        // 잔디는 폭이 커서(최대 31칸) 라벨 없이 한 줄 통째로 — 좁은 카톡 폭(34)에도 들어가게.
        // 더 좁아 다 못 담으면 '가장 중요한 최신 날짜'(뒤)를 보존하며 앞(오래된)부터 줄인다.
        lines.push(content(
            &fit_jandi_tail(&render_jandi(&d.jandi), inner),
            inner,
        ));
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

/// 일요일 저녁 '이번 주 결산' 선톡 문구(데몬이 푸시). 데이터가 없으면 None. 순수.
/// 푸시는 플레인 텍스트라 박스 대신 한 줄 요약 + 카드 보기 유도.
pub fn weekly_recap_text(
    streak: Option<(String, i64)>,
    focus_min: i64,
    focus_delta: i64,
    expense: i64,
    expense_delta: i64,
) -> Option<String> {
    let fmt_h = |m: i64| {
        if m >= 60 {
            format!("{}시간", m / 60)
        } else {
            format!("{m}분")
        }
    };
    let fmt_w = |w: i64| {
        let digits = w.abs().to_string();
        let mut out = String::new();
        for (i, c) in digits.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        out.chars().rev().collect::<String>()
    };
    let mut parts: Vec<String> = Vec::new();
    if let Some((name, s)) = &streak {
        if *s >= 1 {
            parts.push(format!("🔥{name} {s}일"));
        }
    }
    if focus_min > 0 {
        let d = if focus_delta == 0 {
            String::new()
        } else {
            format!("{}{}", delta_arrow(focus_delta), fmt_h(focus_delta.abs()))
        };
        parts.push(format!("🍅집중 {}{d}", fmt_h(focus_min)));
    }
    if expense > 0 {
        let d = if expense_delta == 0 {
            String::new()
        } else {
            format!(
                "{}{}원",
                delta_arrow(expense_delta),
                fmt_w(expense_delta.abs())
            )
        };
        parts.push(format!("💰{}원{d}", fmt_w(expense)));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!(
        "📊 이번 주 결산! {} · 'wonjang 자랑 --주'로 카드 보기 👀",
        parts.join(" · ")
    ))
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
    fn text_presentation_symbols_are_width_one() {
        // 변이선택자 없는 ★♥❤☎ 등은 1칸(터미널·카톡 렌더와 일치). 과거엔 2칸으로 과대계상해 박스가 깨졌다.
        assert_eq!(disp_width("★독서"), 5); // 1 + 2 + 2
        assert_eq!(disp_width("♥운동"), 5);
        assert_eq!(disp_width("❤️운동"), 6); // ❤ + FE0F = 2칸(이모지 표현)
        assert_eq!(disp_width("✍️"), 2); // 카드 라벨(이모지 표현) 보존
                                         // 텍스트표현 기호가 든 습관명으로도 카드 모든 줄이 동일폭.
        let d = CardData {
            title: "2026년 6월".into(),
            streak: Some(("★독서♥".into(), 13)),
            jandi: (0..28).map(|i| i % 2 == 0).collect(),
            focus_label: "3시간".into(),
            expense_label: "1,000원".into(),
            dday: None,
            journal_count: 2,
            comment: "꾸준하네 ☎".into(),
            footer: "wonjang".into(),
        };
        for line in render_card(&d, 40) {
            assert_eq!(disp_width(&line), 40, "★♥☎ 든 카드 줄 폭 불일치: {line:?}");
        }
    }

    #[test]
    fn every_line_has_exact_display_width() {
        // 전각/반각/이모지 혼합 카드의 모든 줄이 정확히 같은 표시폭이어야 박스가 안 깨진다.
        let d = CardData {
            title: "2026년 6월".into(),
            streak: Some(("운동".into(), 100)), // 마일스톤(🏆 뱃지) 포함해 폭 불변식 검증

            jandi: (0..28).map(|i| i % 3 != 0).collect(),
            focus_label: "42시간 30분".into(),
            expense_label: "1,240,000원".into(),
            dday: Some("토익 D-3".into()),
            journal_count: 9,
            comment: "운동 13일 연속, 너 좀 멋진데? 🙌".into(),
            footer: SHARE_FOOTER.into(),
        };
        // 좁은 폭(30~33)도 포함 — 적대적 리뷰가 미커버라 지적한 구간.
        for width in [30usize, 31, 32, 33, 34, 40, 46, 52] {
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
        // 전염성 풋터: 카톡 폭(34)에서 설치명이 잘리면 성장 고리가 끊긴다 → 보존 확인.
        let kakao = render_card(&d, 34);
        let last = kakao.last().unwrap();
        assert!(
            last.contains("wonjang-agent"),
            "카톡 폭에서 풋터 설치명이 잘림: {last:?}"
        );
    }

    #[test]
    fn narrow_jandi_keeps_newest_days() {
        // 최신 날(맨 뒤)만 ▓, 나머지 ░ → 좁은 폭에서 잘려도 최신 ▓가 보존돼야.
        let mut jandi = vec![false; 28];
        jandi[27] = true; // 오늘(최신) 완료
        let d = CardData {
            title: "t".into(),
            streak: None,
            jandi,
            focus_label: "1시간".into(),
            expense_label: "0원".into(),
            dday: None,
            journal_count: 0,
            comment: "x".into(),
            footer: "wonjang".into(),
        };
        let lines = render_card(&d, 32); // inner=30, 잔디 31 > 30 → 앞부터 잘림
        let jandi_line = &lines[1]; // 0=상단테두리, 1=잔디(streak None이라)
        assert!(
            jandi_line.contains('▓'),
            "최신 ▓가 보존돼야: {jandi_line:?}"
        );
        assert_eq!(disp_width(jandi_line), 32);
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
    fn weekly_recap_formats_and_skips_empty() {
        // 데이터 있으면 요약 + 카드 유도.
        let t = weekly_recap_text(Some(("운동".into(), 7)), 660, 120, 240000, -50000).unwrap();
        assert!(t.contains("운동 7일"));
        assert!(t.contains("🍅집중 11시간▲2시간"));
        assert!(t.contains("💰240,000원▼50,000원"));
        assert!(t.contains("자랑 --주"));
        // 변화 0이면 화살표 없음.
        let t2 = weekly_recap_text(None, 120, 0, 0, 0).unwrap();
        assert!(t2.contains("🍅집중 2시간") && !t2.contains("▲") && !t2.contains("▼"));
        // 아무 데이터 없으면 None(선톡 안 함).
        assert!(weekly_recap_text(None, 0, 0, 0, 0).is_none());
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

    #[test]
    fn zwj_emoji_counts_as_single_glyph() {
        // ZWJ 이모지 시퀀스는 한 글자(폭 2) — wcwidth와 일치해야 박스가 안 깨진다.
        assert_eq!(disp_width("🏃\u{200D}♂\u{FE0F}"), 2); // 🏃‍♂️ 달리는 남자
        assert_eq!(disp_width("👨\u{200D}💻"), 2); // 👨‍💻 개발자
        assert_eq!(disp_width("👨\u{200D}👩\u{200D}👧\u{200D}👦"), 2); // 가족(4명 ZWJ)
        // 일반 텍스트·이모지와 섞여도 정확.
        assert_eq!(disp_width("가🏃\u{200D}♂\u{FE0F}나"), 6); // 2+2+2
        // truncate가 ZWJ 글자를 중간에 안 쪼갬: 폭4면 '가'(2)+ZWJ글자(2)=4 다 들어감.
        assert_eq!(truncate_width("가🏃\u{200D}♂\u{FE0F}나", 4), "가🏃\u{200D}♂\u{FE0F}");
    }

    #[test]
    fn streak_badge_tiers() {
        assert_eq!(streak_badge(6), "");
        assert_eq!(streak_badge(7), " 🎉");
        assert_eq!(streak_badge(29), " 🎉");
        assert_eq!(streak_badge(30), " 🎊");
        assert_eq!(streak_badge(100), " 🏆");
        assert_eq!(streak_badge(365), " 👑");
    }

    #[test]
    fn hbar_is_proportional_and_fixed_width() {
        assert_eq!(hbar(10.0, 10.0, 4), "████");
        assert_eq!(hbar(5.0, 10.0, 4), "██  "); // 0.5×4=2칸
        assert_eq!(hbar(0.0, 10.0, 4), "    ");
        assert_eq!(hbar(10.0, 0.0, 4), "    "); // max≤0 가드
        assert_eq!(hbar(99.0, 10.0, 4), "████"); // 비율 1.0으로 클램프
        // 어떤 값이든 정확히 width 표시칸(█·공백 모두 1칸).
        for v in [0.0, 1.0, 3.3, 7.7, 10.0] {
            assert_eq!(disp_width(&hbar(v, 10.0, 8)), 8);
        }
    }
}
