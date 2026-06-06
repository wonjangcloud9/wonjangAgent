//! 날짜 계산 — 두 날짜 사이 일수, N일 후 날짜, 요일 등 즉석 연산.
//!
//! D-day(이름 붙은 이벤트 관리)와 달리 한 번 쓰고 버리는 날짜 산수를 돕는다.
//! 근속일수, 사귄 지 며칠, 마감까지 며칠 같은 계산에 유용하다.

use anyhow::{anyhow, Result};
use chrono::{Datelike, Days, NaiveDate, Weekday};

/// 한국인이 흔히 쓰는 날짜 표기를 표준 `YYYY-MM-DD`로 정규화한다.
/// `2026.11.19`·`2026/11/19`·`2026. 11. 19.`·`20261119`·`2026.1.5`를 모두 받는다.
/// 정규화할 수 없으면 트림한 원본을 그대로 돌려준다(기존 파싱 에러 메시지 유지).
pub fn normalize_date(s: &str) -> String {
    let t = s.trim().trim_end_matches('.').trim();
    // 구분자 없는 8자리(YYYYMMDD).
    if t.len() == 8 && t.bytes().all(|b| b.is_ascii_digit()) {
        return format!("{}-{}-{}", &t[0..4], &t[4..6], &t[6..8]);
    }
    // ./공백을 -로 통일한 뒤 세 토막(연-월-일)이면 0을 채워 표준형으로.
    let unified: String = t
        .chars()
        .map(|c| if matches!(c, '.' | '/' | ' ') { '-' } else { c })
        .collect();
    let parts: Vec<&str> = unified.split('-').filter(|p| !p.is_empty()).collect();
    if parts.len() == 3 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            return format!("{y:04}-{m:02}-{d:02}");
        }
    }
    t.to_string()
}

/// `YYYY-MM-DD` 문자열을 파싱한다(흔한 한국식 표기 `.`·`/`·8자리도 허용).
pub fn parse(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(&normalize_date(s), "%Y-%m-%d")
        .map_err(|_| anyhow!("날짜는 YYYY-MM-DD 형식으로 입력하세요 (예: 2026-01-01)"))
}

/// 요일을 한국어 한 글자로.
pub fn weekday_kr(d: NaiveDate) -> &'static str {
    match d.weekday() {
        Weekday::Mon => "월",
        Weekday::Tue => "화",
        Weekday::Wed => "수",
        Weekday::Thu => "목",
        Weekday::Fri => "금",
        Weekday::Sat => "토",
        Weekday::Sun => "일",
    }
}

/// `a`에서 `b`까지의 일수(b가 미래면 양수).
pub fn days_between(a: NaiveDate, b: NaiveDate) -> i64 {
    (b - a).num_days()
}

/// 기준 날짜에 `n`일을 더한다(음수면 빼기). 표현 가능 범위를 벗어나면 None(패닉 방지).
pub fn add_days(base: NaiveDate, n: i64) -> Option<NaiveDate> {
    if n >= 0 {
        base.checked_add_days(Days::new(n as u64))
    } else {
        base.checked_sub_days(Days::new(n.unsigned_abs()))
    }
}

/// '며칠째'와 다가오는 기념일(사귄 날·기념일 계산용).
pub struct DaysSince {
    pub nth_day: i64,  // 오늘로 N일째(시작일 당일 = 1일째, 한국식)
    pub days_ago: i64, // M일 전
    /// 다가오는 기념일들: (기념일 일수, 그날의 양력 날짜, 오늘로부터 D-day).
    pub milestones: Vec<(i64, NaiveDate, i64)>,
}

/// `start`(과거)부터 `today`까지 며칠째인지와, 커플·기념일에서 챙기는 다가오는 마디 2개.
///
/// 한국식 카운팅: **시작일 당일이 1일째**(그래서 100일은 시작일+99일). 100·1년 등
/// 다음 마디의 양력 날짜를 미리 알려줘 "100일 언제야?"를 한 번에 답한다.
pub fn days_since(start: NaiveDate, today: NaiveDate) -> DaysSince {
    let days_ago = days_between(start, today).max(0);
    let nth_day = days_ago + 1;
    // 100단위 + 해마다(1~10년) 마디를 오름차순으로.
    const MARKS: &[i64] = &[
        100, 200, 300, 365, 500, 730, 1000, 1095, 1460, 1825, 2000, 3000, 3650,
    ];
    let mut milestones = Vec::new();
    for &m in MARKS {
        if m > nth_day {
            // m일째 = 시작일 + (m-1)일.
            if let Some(date) = add_days(start, m - 1) {
                milestones.push((m, date, days_between(today, date)));
                if milestones.len() == 2 {
                    break;
                }
            }
        }
    }
    DaysSince {
        nth_day,
        days_ago,
        milestones,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn diff_counts_days() {
        assert_eq!(days_between(d(2026, 1, 1), d(2026, 12, 31)), 364);
        assert_eq!(days_between(d(2026, 1, 1), d(2025, 12, 31)), -1);
    }

    #[test]
    fn normalize_accepts_common_korean_formats() {
        for s in [
            "2026-11-19",
            "2026.11.19",
            "2026/11/19",
            "2026. 11. 19.",
            "20261119",
            " 2026.11.19 ",
        ] {
            assert_eq!(normalize_date(s), "2026-11-19", "입력: {s:?}");
        }
        // 한 자리 월/일도 0 채움.
        assert_eq!(normalize_date("2026.1.5"), "2026-01-05");
        // 정규화 불가는 트림한 원본 그대로(다운스트림 에러 유지).
        assert_eq!(normalize_date("아무거나"), "아무거나");
        assert_eq!(normalize_date("  11/19  "), "11/19"); // 토막 2개 → 트림 원본 그대로(파싱 실패 유도)
    }

    #[test]
    fn add_and_subtract() {
        assert_eq!(add_days(d(2026, 1, 1), 30), Some(d(2026, 1, 31)));
        assert_eq!(add_days(d(2026, 3, 1), -1), Some(d(2026, 2, 28)));
        assert_eq!(add_days(d(2024, 3, 1), -1), Some(d(2024, 2, 29))); // 윤년
        assert_eq!(add_days(d(2026, 1, 1), i64::MAX), None); // 범위 초과 → None(패닉 없음)
    }

    #[test]
    fn weekday_korean() {
        // 2026-06-01은 월요일.
        assert_eq!(weekday_kr(d(2026, 6, 1)), "월");
    }

    #[test]
    fn days_since_korean_counting() {
        // 2024-01-01 ~ 2026-06-05 = 886일 전, 시작일=1일째 → 887일째.
        let s = days_since(d(2024, 1, 1), d(2026, 6, 5));
        assert_eq!(s.days_ago, 886);
        assert_eq!(s.nth_day, 887);
        // 다가오는 마디 2개: 1000일, 1095일(3년).
        assert_eq!(s.milestones.len(), 2);
        assert_eq!(s.milestones[0].0, 1000);
        assert_eq!(s.milestones[1].0, 1095);
        // 1000일째 = 시작일 + 999일, D-day는 양수(미래).
        assert_eq!(s.milestones[0].1, add_days(d(2024, 1, 1), 999).unwrap());
        assert!(s.milestones[0].2 > 0);
    }

    #[test]
    fn days_since_100day_couple() {
        // 6/1 시작, 6/5 = 5일째. 다음 마디 100일 = 시작일 + 99일.
        let s = days_since(d(2026, 6, 1), d(2026, 6, 5));
        assert_eq!(s.nth_day, 5);
        assert_eq!(s.milestones[0].0, 100);
        assert_eq!(s.milestones[0].1, add_days(d(2026, 6, 1), 99).unwrap());
    }
}
