//! 시간 계산 — `09:00 + 8:30` 같은 시·분 산수.
//!
//! 근무시간 합산, 회의·영상 길이 더하기 등에 쓴다. 각 항목을 분으로 바꿔
//! 부호대로 더한다. 순수 계산이라 키가 없다.

use anyhow::{anyhow, Result};

/// "H:MM" 또는 "HH:MM"(또는 분 단위 정수)을 분으로 바꾼다.
fn parse_minutes(tok: &str) -> Result<i64> {
    if let Some((h, m)) = tok.split_once(':') {
        let h: i64 = h
            .trim()
            .parse()
            .map_err(|_| anyhow!("시간 형식이 올바르지 않아요: {tok}"))?;
        let m: i64 = m
            .trim()
            .parse()
            .map_err(|_| anyhow!("시간 형식이 올바르지 않아요: {tok}"))?;
        if !(0..60).contains(&m) {
            return Err(anyhow!("분은 0~59 사이여야 해요: {tok}"));
        }
        Ok(h * 60 + m)
    } else {
        // 콜론이 없으면 분 단위 숫자로 본다.
        tok.trim()
            .parse::<i64>()
            .map_err(|_| anyhow!("시간(H:MM) 또는 분(숫자)을 입력하세요: {tok}"))
    }
}

/// 토큰 목록(시간/부호)을 더해 총 분을 구한다.
///
/// 첫 항목은 더하기. 이후 `+`/`-` 토큰이 다음 항목의 부호를 정한다.
pub fn sum(tokens: &[String]) -> Result<i64> {
    let mut total = 0i64;
    let mut sign = 1i64;
    let mut seen = false;
    for tok in tokens {
        match tok.as_str() {
            "+" => sign = 1,
            "-" => sign = -1,
            t => {
                total += sign * parse_minutes(t)?;
                sign = 1; // 다음 기본값은 더하기
                seen = true;
            }
        }
    }
    if !seen {
        return Err(anyhow!("계산할 시간을 입력하세요. 예: 09:00 + 8:30"));
    }
    Ok(total)
}

/// 분을 "X시간 Y분"으로(음수면 부호 포함).
pub fn format_hm(total: i64) -> String {
    let neg = total < 0;
    let abs = total.abs();
    let h = abs / 60;
    let m = abs % 60;
    let sign = if neg { "-" } else { "" };
    if h > 0 && m > 0 {
        format!("{sign}{h}시간 {m}분")
    } else if h > 0 {
        format!("{sign}{h}시간")
    } else {
        format!("{sign}{m}분")
    }
}

/// 분을 "HH:MM" 24시간 시계 형식으로(하루 단위로 감싼다).
pub fn format_clock(total: i64) -> String {
    let wrapped = total.rem_euclid(24 * 60);
    format!("{:02}:{:02}", wrapped / 60, wrapped % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn add_times() {
        assert_eq!(sum(&toks("09:00 + 8:30")).unwrap(), 17 * 60 + 30);
        assert_eq!(sum(&toks("1:30 + 2:45 + 0:50")).unwrap(), 5 * 60 + 5);
    }

    #[test]
    fn subtract_times() {
        assert_eq!(sum(&toks("18:00 - 9:30")).unwrap(), 8 * 60 + 30);
        assert_eq!(sum(&toks("1:00 - 1:30")).unwrap(), -30);
    }

    #[test]
    fn formats() {
        assert_eq!(format_hm(305), "5시간 5분");
        assert_eq!(format_hm(120), "2시간");
        assert_eq!(format_hm(-30), "-30분");
        assert_eq!(format_clock(17 * 60 + 30), "17:30");
        assert_eq!(format_clock(25 * 60), "01:00"); // 하루 감쌈
    }

    #[test]
    fn rejects_bad_input() {
        assert!(sum(&toks("9:70")).is_err());
        assert!(sum(&toks("abc")).is_err());
        assert!(sum(&[]).is_err());
    }
}
