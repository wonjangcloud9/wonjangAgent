//! 날짜 계산 — 두 날짜 사이 일수, N일 후 날짜, 요일 등 즉석 연산.
//!
//! D-day(이름 붙은 이벤트 관리)와 달리 한 번 쓰고 버리는 날짜 산수를 돕는다.
//! 근속일수, 사귄 지 며칠, 마감까지 며칠 같은 계산에 유용하다.

use anyhow::{anyhow, Result};
use chrono::{Datelike, Days, NaiveDate, Weekday};

/// `YYYY-MM-DD` 문자열을 파싱한다.
pub fn parse(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
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
}
