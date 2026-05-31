//! 일일 자동 브리핑 — 스케줄러 데몬이 매일 아침 정해진 시각에 브리핑을 만들어
//! 설정된 채널(카카오/디스코드/텔레그램)로 자동 푸시한다.
//!
//! 여기서는 "지금이 브리핑할 시각인가"를 판정하는 순수 로직만 둔다(테스트 용이).
//! 실제 실행(LLM·푸시)은 cron 데몬 루프에서 이 판정을 사용한다.

/// "HH:MM" 문자열을 (시, 분)으로 파싱.
pub fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let s = s.trim();
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h < 24 && m < 60 {
        Some((h, m))
    } else {
        None
    }
}

/// 오늘 브리핑을 보낼 때인가.
///
/// - `briefing_time`이 비었거나 형식이 틀리면 false(비활성).
/// - 오늘 이미 보냈으면(`last_date == today`) false.
/// - 현재 시각이 목표 시각을 지났으면 true.
pub fn should_brief(
    briefing_time: &str,
    last_date: Option<&str>,
    today: &str,
    now_h: u32,
    now_m: u32,
) -> bool {
    let (h, m) = match parse_hhmm(briefing_time) {
        Some(t) => t,
        None => return false,
    };
    if last_date == Some(today) {
        return false;
    }
    now_h > h || (now_h == h && now_m >= m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cases() {
        assert_eq!(parse_hhmm("08:00"), Some((8, 0)));
        assert_eq!(parse_hhmm("23:59"), Some((23, 59)));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("8시"), None);
        assert_eq!(parse_hhmm(""), None);
    }

    #[test]
    fn disabled_when_empty() {
        assert!(!should_brief("", None, "2026-06-01", 9, 0));
    }

    #[test]
    fn briefs_after_time_once_per_day() {
        // 08:00 설정, 지금 08:30, 아직 안 보냄 → 보냄.
        assert!(should_brief("08:00", None, "2026-06-01", 8, 30));
        // 이미 오늘 보냄 → 안 보냄.
        assert!(!should_brief(
            "08:00",
            Some("2026-06-01"),
            "2026-06-01",
            8,
            30
        ));
        // 아직 시각 전(07:59) → 안 보냄.
        assert!(!should_brief("08:00", None, "2026-06-01", 7, 59));
        // 어제 보냈고 오늘 시각 지남 → 보냄.
        assert!(should_brief(
            "08:00",
            Some("2026-05-31"),
            "2026-06-01",
            8,
            0
        ));
    }
}
