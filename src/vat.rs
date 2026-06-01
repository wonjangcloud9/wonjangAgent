//! 부가가치세(VAT) 계산 — 공급가액과 세액을 분리/합산한다.
//!
//! 한국 부가세율은 10%. 입력 금액이 '공급가액'일 때와 'VAT 포함 합계'일 때를
//! 모두 보여줘 헷갈림을 줄인다. 순수 계산이라 키가 없다.

/// 부가세율(10%).
pub const VAT_RATE: f64 = 0.10;

/// 금액을 공급가액으로 볼 때의 (공급가, 세액, 합계).
pub fn from_supply(supply: f64) -> (f64, f64, f64) {
    let vat = supply * VAT_RATE;
    (supply, vat, supply + vat)
}

/// 금액을 VAT 포함 합계로 볼 때의 (공급가, 세액, 합계).
pub fn from_total(total: f64) -> (f64, f64, f64) {
    let supply = total / (1.0 + VAT_RATE);
    (supply, total - supply, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supply_adds_10_percent() {
        // 공급가 100,000 → 세액 10,000, 합계 110,000.
        let (s, v, t) = from_supply(100_000.0);
        assert_eq!(s, 100_000.0);
        assert!((v - 10_000.0).abs() < 0.01);
        assert!((t - 110_000.0).abs() < 0.01);
    }

    #[test]
    fn total_extracts_supply() {
        // 합계 110,000 → 공급가 100,000, 세액 10,000.
        let (s, v, t) = from_total(110_000.0);
        assert!((s - 100_000.0).abs() < 0.01);
        assert!((v - 10_000.0).abs() < 0.01);
        assert_eq!(t, 110_000.0);
    }

    #[test]
    fn round_trip() {
        let (_, _, total) = from_supply(33_333.0);
        let (supply, _, _) = from_total(total);
        assert!((supply - 33_333.0).abs() < 0.01);
    }
}
