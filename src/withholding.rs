//! 원천징수 계산 — 프리랜서·개인사업자 사업소득 3.3% / 기타소득 8.8%.
//!
//! 사업소득: 소득세 3% + 지방소득세(소득세의 10%) = 3.3%.
//! 기타소득(강연료·원고료 등): 소득세 8% + 지방세 0.8% = 8.8%(필요경비 60% 가정한 표준).
//! 지급액→실수령(정산)과 실수령→세전(역산) 모두 — 역산은 `실수령×1.033` 오답을 바로잡는다.
//! 세율 고정·정수 연산이라 검증 가능. (단, 역산은 정수 절사로 ±1원 추정.)

/// 원천징수 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Withholding {
    pub gross: i64,      // 세전 지급액
    pub income_tax: i64, // 소득세
    pub local_tax: i64,  // 지방소득세
    pub total: i64,      // 총 원천징수액
    pub net: i64,        // 실수령액
}

/// 소득세율(%) — 사업소득 3, 기타소득 8.
fn income_pct(etc: bool) -> i64 {
    if etc {
        8
    } else {
        3
    }
}

/// 지급액(세전) → 원천징수·실수령.
pub fn from_gross(gross: i64, etc: bool) -> Withholding {
    let income_tax = gross * income_pct(etc) / 100;
    let local_tax = income_tax / 10; // 지방소득세 = 소득세의 10%
    let total = income_tax + local_tax;
    Withholding {
        gross,
        income_tax,
        local_tax,
        total,
        net: gross - total,
    }
}

/// 실수령액 → 세전 지급액(역산) 후 정산. `gross = net / (1 - 세율)` 반올림.
pub fn from_net(net: i64, etc: bool) -> Withholding {
    // 세율(천분율): 3.3% = 33, 8.8% = 88.
    let permille = if etc { 88 } else { 33 };
    let denom = 1000 - permille;
    let gross = (net * 1000 + denom / 2) / denom; // 반올림
    from_gross(gross, etc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_3_3_percent() {
        let w = from_gross(1_000_000, false);
        assert_eq!(w.income_tax, 30_000);
        assert_eq!(w.local_tax, 3_000);
        assert_eq!(w.total, 33_000);
        assert_eq!(w.net, 967_000);
    }

    #[test]
    fn etc_8_8_percent() {
        let w = from_gross(1_000_000, true);
        assert_eq!(w.total, 88_000);
        assert_eq!(w.net, 912_000);
    }

    #[test]
    fn reverse_recovers_gross() {
        // 실수령 967,000 → 세전 1,000,000(정확).
        assert_eq!(from_net(967_000, false).gross, 1_000_000);
        assert_eq!(from_net(912_000, true).gross, 1_000_000);
        // 흔한 오답 방지: 실수령 100만의 세전은 103.3만이 아니라 약 103.4만.
        let g = from_net(1_000_000, false).gross;
        assert!((1_034_000..=1_034_200).contains(&g), "g={g}");
    }
}
