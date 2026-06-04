//! 유닉스 타임스탬프 변환 — 로그·API의 타임스탬프 ↔ 사람이 읽는 시각.
//!
//! "이 timestamp 1717200000 언제야?", "지금 유닉스 타임 뭐야?"를 처리한다.
//! 현재 시각은 GPT가 알 수 없으므로 직접 시스템 시계를 읽는다. chrono만 쓴다.

use anyhow::{anyhow, Result};
use chrono::{Local, TimeZone, Utc};

/// 변환 결과(사람이 읽는 형태).
pub struct Converted {
    pub unix_sec: i64,
    pub local: String,
    pub utc: String,
}

fn format_from_unix(sec: i64) -> Result<Converted> {
    let local = Local
        .timestamp_opt(sec, 0)
        .single()
        .ok_or_else(|| anyhow!("타임스탬프 범위를 벗어났어요: {sec}"))?;
    let utc = Utc
        .timestamp_opt(sec, 0)
        .single()
        .ok_or_else(|| anyhow!("타임스탬프 범위를 벗어났어요: {sec}"))?;
    Ok(Converted {
        unix_sec: sec,
        // 요일은 한국어로(예전 `%a`는 "(Fri)" 같은 영문이라 한국어 출력에 이질적).
        local: format!(
            "{} ({})",
            local.format("%Y-%m-%d %H:%M:%S"),
            crate::datecalc::weekday_kr(local.date_naive())
        ),
        utc: utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    })
}

/// 입력을 해석한다. 숫자면 유닉스(초/밀리초), 날짜 문자열이면 그 시각의 유닉스.
pub fn convert(input: &str) -> Result<Converted> {
    let s = input.trim();
    // 전부 숫자(또는 음수) → 유닉스 타임스탬프.
    if s.chars()
        .enumerate()
        .all(|(i, c)| c.is_ascii_digit() || (i == 0 && c == '-'))
    {
        let mut n: i64 = s.parse().map_err(|_| anyhow!("숫자가 너무 커요: {s}"))?;
        // 13자리 이상이면 밀리초로 보고 초로 변환.
        if s.trim_start_matches('-').len() >= 13 {
            n /= 1000;
        }
        return format_from_unix(n);
    }
    // 날짜 문자열 → 유닉스.
    let dt = parse_datetime(s)?;
    format_from_unix(dt)
}

/// 현재 시각을 변환 결과로.
pub fn now() -> Converted {
    let sec = Local::now().timestamp();
    // now()는 항상 유효 범위.
    format_from_unix(sec).unwrap_or(Converted {
        unix_sec: sec,
        local: String::new(),
        utc: String::new(),
    })
}

/// "YYYY-MM-DD" 또는 "YYYY-MM-DD HH:MM:SS"를 로컬 기준 유닉스 초로.
fn parse_datetime(s: &str) -> Result<i64> {
    use chrono::NaiveDateTime;
    let naive = if s.contains(' ') || s.contains('T') {
        let s = s.replace('T', " ");
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| anyhow!("날짜 형식: YYYY-MM-DD 또는 YYYY-MM-DD HH:MM:SS"))?
    } else {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| anyhow!("날짜 형식: YYYY-MM-DD"))?
            .and_hms_opt(0, 0, 0)
            .unwrap()
    };
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.timestamp())
        .ok_or_else(|| anyhow!("로컬 시각으로 변환할 수 없어요"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_to_readable() {
        // 1700000000 = 2023-11-14 22:13:20 UTC.
        let c = convert("1700000000").unwrap();
        assert_eq!(c.unix_sec, 1700000000);
        assert!(c.utc.starts_with("2023-11-14"));
    }

    #[test]
    fn milliseconds_detected() {
        let c = convert("1700000000000").unwrap();
        assert_eq!(c.unix_sec, 1700000000);
    }

    #[test]
    fn date_to_unix_roundtrip() {
        let c = convert("2023-11-14").unwrap();
        // 같은 날짜로 다시 변환하면 자정.
        assert!(c.local.starts_with("2023-11-14 00:00:00"));
        // 요일은 한국어로(2023-11-14는 화요일) — 영문 "(Tue)"가 아니어야.
        assert!(c.local.contains("(화)"), "한국어 요일이어야: {}", c.local);
        assert!(!c.local.contains("Tue"));
    }

    #[test]
    fn rejects_garbage() {
        assert!(convert("not-a-date").is_err());
    }
}
