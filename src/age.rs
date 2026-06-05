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

/// 세는나이(한국식): 태어나면 1세, 해 바뀔 때 +1 = 현재 연도 − 출생 연도 + 1.
/// 2023년 '만 나이 통일법'으로 공식 폐지됐지만 일상 대화("우리 나이로 몇 살")에선 여전히 쓰인다.
pub fn counting_age(birth: NaiveDate, today: NaiveDate) -> i32 {
    today.year() - birth.year() + 1
}

/// 출생 연도의 십이지 띠.
pub fn zodiac_animal(year: i32) -> &'static str {
    const ANIMALS: [&str; 12] = [
        "쥐",
        "소",
        "호랑이",
        "토끼",
        "용",
        "뱀",
        "말",
        "양",
        "원숭이",
        "닭",
        "개",
        "돼지",
    ];
    // 서기 4년이 쥐(자)년 기준. 음수 연도는 다루지 않는다.
    let idx = ((year - 4).rem_euclid(12)) as usize;
    ANIMALS[idx]
}

/// 생월·생일로 서양 12별자리.
pub fn star_sign(month: u32, day: u32) -> &'static str {
    // (시작월, 시작일, 이름) — 해당 시작일 이후 구간.
    match (month, day) {
        (1, d) if d <= 19 => "염소자리",
        (1, _) | (2, ..=18) => "물병자리",
        (2, _) | (3, ..=20) => "물고기자리",
        (3, _) | (4, ..=19) => "양자리",
        (4, _) | (5, ..=20) => "황소자리",
        (5, _) | (6, ..=21) => "쌍둥이자리",
        (6, _) | (7, ..=22) => "게자리",
        (7, _) | (8, ..=22) => "사자자리",
        (8, _) | (9, ..=22) => "처녀자리",
        (9, _) | (10, ..=22) => "천칭자리",
        (10, _) | (11, ..=22) => "전갈자리",
        (11, _) | (12, ..=21) => "사수자리",
        _ => "염소자리", // 12/22~12/31
    }
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

/// 태어난 지 며칠(살아온 날수). 백일·천일처럼 한국에서 챙기는 '날수' 기념의 토대.
pub fn days_lived(birth: NaiveDate, today: NaiveDate) -> i64 {
    (today - birth).num_days()
}

/// 다음 1000일 단위 기념일: `(그 날수, 양력 날짜)`. 살아온 날이 음수거나 범위를 벗어나면 None.
pub fn next_day_milestone(birth: NaiveDate, today: NaiveDate) -> Option<(i64, NaiveDate)> {
    let lived = days_lived(birth, today);
    if lived < 0 {
        return None;
    }
    let next = (lived / 1000 + 1) * 1000;
    let date = birth.checked_add_days(chrono::Days::new(next as u64))?;
    Some((next, date))
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
    fn counting_age_is_year_age_plus_one() {
        // 1990년생, 2026년 → 세는나이 37(= 연나이 36 + 1).
        assert_eq!(counting_age(d(1990, 5, 15), d(2026, 6, 1)), 37);
        // 생일 전이어도 세는나이는 같음(연도 기준).
        assert_eq!(counting_age(d(1990, 12, 31), d(2026, 1, 1)), 37);
    }

    #[test]
    fn countdown() {
        assert_eq!(days_to_birthday(d(1990, 3, 15), d(2026, 3, 15)), 0);
        assert_eq!(days_to_birthday(d(1990, 3, 15), d(2026, 3, 14)), 1);
    }

    #[test]
    fn zodiac() {
        assert_eq!(zodiac_animal(1990), "말"); // 경오년
        assert_eq!(zodiac_animal(2008), "쥐");
        assert_eq!(zodiac_animal(2007), "돼지");
    }

    #[test]
    fn star_signs() {
        assert_eq!(star_sign(3, 15), "물고기자리");
        assert_eq!(star_sign(3, 21), "양자리");
        assert_eq!(star_sign(1, 19), "염소자리");
        assert_eq!(star_sign(1, 20), "물병자리");
        assert_eq!(star_sign(12, 25), "염소자리");
    }

    #[test]
    fn lived_and_milestone() {
        let birth = d(1990, 5, 15);
        let today = d(2026, 6, 6);
        let lived = days_lived(birth, today);
        assert!((13000..13300).contains(&lived), "lived={lived}");
        let (mark, date) = next_day_milestone(birth, today).unwrap();
        assert_eq!(mark % 1000, 0);
        assert!(mark > lived && mark - lived <= 1000); // 다음 마디는 한 1000일 안쪽
                                                       // 마디 날짜 = 생일 + mark일.
        assert_eq!(
            date,
            birth
                .checked_add_days(chrono::Days::new(mark as u64))
                .unwrap()
        );
        // 미래 생일은 None(살아온 날 음수).
        assert!(next_day_milestone(d(2030, 1, 1), today).is_none());
    }
}
