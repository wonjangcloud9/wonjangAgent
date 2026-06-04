//! 환율 — 해외직구·여행으로 자주 보는 환율을 실시간으로(무료, 키 불필요).
//!
//! open.er-api.com(USD 기준)을 받아 원화 환산값을 계산한다.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Resp {
    #[serde(default)]
    result: String,
    #[serde(default)]
    time_last_update_utc: String,
    #[serde(default)]
    rates: HashMap<String, f64>,
}

/// USD 기준 환율표와 갱신 시각을 가져온다.
pub async fn fetch() -> Result<(String, HashMap<String, f64>)> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let r: Resp = http
        .get("https://open.er-api.com/v6/latest/USD")
        .send()
        .await
        .context("환율 요청 실패")?
        .json()
        .await
        .context("환율 응답 파싱 실패")?;
    if r.result != "success" || r.rates.is_empty() {
        bail!("환율 데이터를 가져오지 못했습니다");
    }
    Ok((r.time_last_update_utc, r.rates))
}

/// 통화 1단위가 몇 원인지(USD 기준표에서 계산).
pub fn krw_per(currency: &str, rates: &HashMap<String, f64>) -> Option<f64> {
    let cur = currency.to_uppercase();
    if cur == "KRW" {
        return Some(1.0);
    }
    let krw = rates.get("KRW")?;
    let c = rates.get(&cur)?;
    if *c == 0.0 {
        return None;
    }
    Some(krw / c)
}

/// 갱신 시각(API가 주는 RFC-2822·UTC 문자열)을 한국 시간 기준 깔끔한 표기로.
///
/// open.er-api.com은 `"Thu, 04 Jun 2026 00:02:31 +0000"`처럼 **영문 RFC-2822**로
/// 내려보내는데, 한국어 일색인 출력에 영문 날짜가 끼면 이질적이다. 한국 사용자에게
/// 가장 쓸모 있는 **KST 기준**으로 `"2026-06-04 09:02 기준 (KST)"`로 바꾼다.
/// 파싱 실패 시(형식 변화 등) 원본을 그대로 돌려준다.
pub fn format_update_time(rfc2822: &str) -> String {
    use chrono::{DateTime, FixedOffset};
    let s = rfc2822.trim();
    match DateTime::parse_from_rfc2822(s) {
        Ok(dt) => {
            let kst = dt.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
            format!("{} 기준 (KST)", kst.format("%Y-%m-%d %H:%M"))
        }
        Err(_) => s.to_string(),
    }
}

/// 통화 코드의 한국어 이름.
pub fn currency_name(code: &str) -> &'static str {
    match code.to_uppercase().as_str() {
        "USD" => "달러",
        "JPY" => "엔",
        "EUR" => "유로",
        "CNY" => "위안",
        "GBP" => "파운드",
        "HKD" => "홍콩달러",
        "AUD" => "호주달러",
        "VND" => "동",
        "THB" => "바트",
        _ => "",
    }
}

/// 숫자를 천 단위 콤마 + 소수 옵션으로.
pub fn comma(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    let (int_part, frac) = match s.split_once('.') {
        Some((i, f)) => (i.to_string(), Some(f.to_string())),
        None => (s, None),
    };
    let neg = int_part.starts_with('-');
    let digits = int_part.trim_start_matches('-');
    let mut out = String::new();
    let bytes = digits.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    let mut result = if neg { format!("-{out}") } else { out };
    if let Some(f) = frac {
        result.push('.');
        result.push_str(&f);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rates() -> HashMap<String, f64> {
        // USD 기준: 1 USD = 1500 KRW = 150 JPY = 0.9 EUR
        [
            ("KRW".to_string(), 1500.0),
            ("JPY".to_string(), 150.0),
            ("EUR".to_string(), 0.9),
            ("USD".to_string(), 1.0),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn krw_conversion() {
        let r = rates();
        assert_eq!(krw_per("USD", &r), Some(1500.0));
        assert_eq!(krw_per("JPY", &r), Some(10.0)); // 1500/150
        assert_eq!(krw_per("KRW", &r), Some(1.0));
        assert_eq!(krw_per("XXX", &r), None);
    }

    #[test]
    fn comma_format() {
        assert_eq!(comma(1506.777, 2), "1,506.78");
        assert_eq!(comma(150678.0, 0), "150,678");
        assert_eq!(comma(946.0, 0), "946");
    }

    #[test]
    fn update_time_to_kst() {
        // UTC 00:02 → KST 09:02.
        assert_eq!(
            format_update_time("Thu, 04 Jun 2026 00:02:31 +0000"),
            "2026-06-04 09:02 기준 (KST)"
        );
        // 자정 직전 UTC는 다음 날 KST로 넘어간다.
        assert_eq!(
            format_update_time("Wed, 03 Jun 2026 23:30:00 +0000"),
            "2026-06-04 08:30 기준 (KST)"
        );
        // 파싱 실패 시 원본 보존(형식 변화 방어).
        assert_eq!(format_update_time("garbage"), "garbage");
    }
}
