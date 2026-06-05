//! 전월세 전환 계산 — 전세보증금을 (반)월세로 돌릴 때의 월세를 구한다.
//!
//! 월세 = (전세보증금 − 월세보증금) × 전월세전환율 / 12. 전환율은 법정 상한
//! (한국은행 기준금리 + 2%)을 넘을 수 없다. 순수 공식이라 키가 필요 없다.

/// 전세보증금을 (반)월세로 전환했을 때의 **월세(만원)**.
/// 월세 = (전세보증금 − 월세보증금) × 전환율/100 / 12.
pub fn monthly_rent(jeonse_manwon: f64, deposit_manwon: f64, rate_pct: f64) -> f64 {
    let converted = (jeonse_manwon - deposit_manwon).max(0.0);
    converted * (rate_pct / 100.0) / 12.0
}

/// 천 단위 콤마(러스트 포맷엔 콤마 그룹화가 없어 직접).
fn commas(n: i64) -> String {
    let s = n.abs().to_string();
    let b = s.as_bytes();
    let mut out = String::new();
    for (i, c) in b.iter().enumerate() {
        if i > 0 && (b.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*c as char);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// 만원 금액을 'X억 Y만원' 한국식 표기로. 0이면 "0원".
pub fn fmt_eok(manwon: f64) -> String {
    let m = manwon.round() as i64;
    if m == 0 {
        return "0원".to_string();
    }
    let eok = m / 10_000;
    let man = m % 10_000;
    match (eok, man) {
        (0, man) => format!("{}만원", commas(man)),
        (eok, 0) => format!("{}억", commas(eok)),
        (eok, man) => format!("{}억 {}만원", commas(eok), commas(man)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_jeonse_to_banwolse() {
        // 전세 3억(30000만), 보증금 1억(10000만), 5.5% → (20000)×0.055/12 = 91.666…만원.
        let m = monthly_rent(30000.0, 10000.0, 5.5);
        assert!((m - 91.6667).abs() < 0.01, "{m}");
    }

    #[test]
    fn full_monthly_when_no_deposit() {
        // 보증금 0 → 30000×0.055/12 = 137.5만원.
        assert!((monthly_rent(30000.0, 0.0, 5.5) - 137.5).abs() < 0.001);
    }

    #[test]
    fn eok_format() {
        assert_eq!(fmt_eok(30000.0), "3억");
        assert_eq!(fmt_eok(35000.0), "3억 5,000만원");
        assert_eq!(fmt_eok(8000.0), "8,000만원");
        assert_eq!(fmt_eok(0.0), "0원");
    }
}
