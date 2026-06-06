//! 월급날 카운트다운 — 한국 직장인이 가장 사랑하는 D-day.
//!
//! 월급날은 *매달 반복*이라 일회성 `디데이`로는 안 맞는다. `월급날 25`처럼
//! 지정하면 이번 달(아직이면)·다음 달(지났으면) 다음 월급날까지 며칠을 계산한다.
//! 말일 지급(`월급날 말일`)과 그 달에 없는 날(예: 31일 → 30일 달이면 말일)도 처리.
//! 순수 날짜 계산이라 키·네트워크가 없다.

use chrono::{Datelike, Duration, NaiveDate};

/// 월급날 지정.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Payday {
    /// 매달 N일(그 달에 없으면 말일로 클램프: 31일 지정 → 30일 달이면 30일).
    Day(u32),
    /// 말일(매달 마지막 날).
    LastDay,
}

/// `"25"`·`"말일"`·`"31"` 같은 입력을 `Payday`로 파싱한다.
pub fn parse(spec: &str) -> Result<Payday, String> {
    let t = spec.trim().trim_end_matches('일').trim();
    if matches!(t, "말" | "말일" | "마지막" | "last") {
        return Ok(Payday::LastDay);
    }
    match t.parse::<u32>() {
        Ok(d) if (1..=31).contains(&d) => Ok(Payday::Day(d)),
        _ => Err(format!(
            "월급날은 1~31 또는 '말일'로 입력하세요 (예: 월급날 25, 월급날 말일). 입력: '{spec}'"
        )),
    }
}

fn last_day_of_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    // 다음 달 1일에서 하루 빼면 이번 달 말일.
    (NaiveDate::from_ymd_opt(ny, nm, 1).unwrap() - Duration::days(1)).day()
}

fn payday_in_month(spec: &Payday, y: i32, m: u32) -> NaiveDate {
    let last = last_day_of_month(y, m);
    let day = match spec {
        Payday::LastDay => last,
        Payday::Day(d) => (*d).min(last), // 그 달에 없는 날은 말일로
    };
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// 오늘(포함) 기준 다음 월급날.
pub fn next_payday(spec: &Payday, today: NaiveDate) -> NaiveDate {
    let this = payday_in_month(spec, today.year(), today.month());
    if this >= today {
        return this;
    }
    let (y, m) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    payday_in_month(spec, y, m)
}

/// 월급날이 주말이면 보통 앞 영업일(금)에 지급된다 — 그 금요일을 돌려준다(평일이면 None).
/// 공휴일은 별도 데이터가 필요해 여기선 주말만 본다(안내는 '보통'으로 단정하지 않음).
pub fn weekend_early_payday(date: NaiveDate) -> Option<NaiveDate> {
    use chrono::Weekday;
    match date.weekday() {
        Weekday::Sat => Some(date - Duration::days(1)),
        Weekday::Sun => Some(date - Duration::days(2)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn parses_day_and_lastday() {
        assert_eq!(parse("25"), Ok(Payday::Day(25)));
        assert_eq!(parse("25일"), Ok(Payday::Day(25)));
        assert_eq!(parse("말일"), Ok(Payday::LastDay));
        assert!(parse("0").is_err());
        assert!(parse("32").is_err());
        assert!(parse("내일").is_err());
    }

    #[test]
    fn next_payday_rolls_over_after_passing() {
        // 6/6 기준 이번 달 25일이 다음 월급날.
        assert_eq!(next_payday(&Payday::Day(25), d(2026, 6, 6)), d(2026, 6, 25));
        // 당일이면 오늘.
        assert_eq!(
            next_payday(&Payday::Day(25), d(2026, 6, 25)),
            d(2026, 6, 25)
        );
        // 지났으면 다음 달.
        assert_eq!(
            next_payday(&Payday::Day(25), d(2026, 6, 26)),
            d(2026, 7, 25)
        );
        // 연말 → 다음 해.
        assert_eq!(
            next_payday(&Payday::Day(10), d(2026, 12, 20)),
            d(2027, 1, 10)
        );
    }

    #[test]
    fn clamps_day_not_in_month() {
        // 31일 지정 + 6월(30일) → 6/30.
        assert_eq!(next_payday(&Payday::Day(31), d(2026, 6, 6)), d(2026, 6, 30));
        // 31일 + 2월(28일) → 2/28.
        assert_eq!(
            next_payday(&Payday::Day(31), d(2026, 2, 10)),
            d(2026, 2, 28)
        );
        // 윤년 2월 → 2/29.
        assert_eq!(
            next_payday(&Payday::Day(31), d(2024, 2, 10)),
            d(2024, 2, 29)
        );
    }

    #[test]
    fn last_day_payday() {
        assert_eq!(next_payday(&Payday::LastDay, d(2026, 6, 6)), d(2026, 6, 30));
        assert_eq!(next_payday(&Payday::LastDay, d(2026, 2, 1)), d(2026, 2, 28));
    }

    #[test]
    fn weekend_payday_points_to_friday() {
        // 2026-06-27은 토요일 → 앞 금요일 06-26.
        assert_eq!(weekend_early_payday(d(2026, 6, 27)), Some(d(2026, 6, 26)));
        // 2026-06-28은 일요일 → 앞 금요일 06-26.
        assert_eq!(weekend_early_payday(d(2026, 6, 28)), Some(d(2026, 6, 26)));
        // 평일(목요일 06-25)이면 None.
        assert_eq!(weekend_early_payday(d(2026, 6, 25)), None);
    }
}
