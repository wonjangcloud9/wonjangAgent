//! 만 나이 계산.
//!
//! 2023년 6월 시행된 '만 나이 통일법' 기준으로 만 나이를 계산한다.
//! 한국에서는 만 나이 외에 '연 나이'(현재 연도 − 출생 연도)도 행정상
//! 쓰이므로 두 값을 함께 제공한다.

use anyhow::{anyhow, Result};
use chrono::{Datelike, NaiveDate};

/// `YYYY-MM-DD` 형식의 생일 문자열을 파싱한다.
pub fn parse_birth(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| anyhow!("생일은 YYYY-MM-DD 형식으로 입력하세요 (예: 1990-03-15)"))
}

/// 기준일(`today`) 시점의 만 나이.
///
/// 생일이 아직 지나지 않았으면 1을 빼는, 국제 표준 만 나이 계산.
pub fn korean_age(birth: NaiveDate, today: NaiveDate) -> i32 {
    let mut age = today.year() - birth.year();
    // 올해 생일이 아직 안 지났으면 한 살 빼기.
    if (today.month(), today.day()) < (birth.month(), birth.day()) {
        age -= 1;
    }
    age
}

/// 연 나이(현재 연도 − 출생 연도). 병역·청소년보호법 등에서 쓰인다.
pub fn year_age(birth: NaiveDate, today: NaiveDate) -> i32 {
    today.year() - birth.year()
}

/// 다음 생일까지 남은 일수. 오늘이 생일이면 0.
pub fn days_to_birthday(birth: NaiveDate, today: NaiveDate) -> i64 {
    let mut next = NaiveDate::from_ymd_opt(today.year(), birth.month(), birth.day())
        // 2월 29일생 등 올해 해당 날짜가 없으면 3월 1일로 대체.
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(today.year(), 3, 1).unwrap());
    if next < today {
        next = NaiveDate::from_ymd_opt(today.year() + 1, birth.month(), birth.day())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(today.year() + 1, 3, 1).unwrap());
    }
    (next - today).num_days()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn birthday_not_yet_passed() {
        // 1990-03-15생, 오늘 2026-03-14 → 만 35세(생일 하루 전).
        assert_eq!(korean_age(d(1990, 3, 15), d(2026, 3, 14)), 35);
        // 생일 당일이면 만 36세.
        assert_eq!(korean_age(d(1990, 3, 15), d(2026, 3, 15)), 36);
        // 생일 다음 날도 만 36세.
        assert_eq!(korean_age(d(1990, 3, 15), d(2026, 3, 16)), 36);
    }

    #[test]
    fn year_age_ignores_month() {
        assert_eq!(year_age(d(1990, 12, 31), d(2026, 1, 1)), 36);
    }

    #[test]
    fn countdown() {
        assert_eq!(days_to_birthday(d(1990, 3, 15), d(2026, 3, 15)), 0);
        assert_eq!(days_to_birthday(d(1990, 3, 15), d(2026, 3, 14)), 1);
    }
}
