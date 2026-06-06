//! 종합소득세(개인사업자·프리랜서) 추정 — 과세표준에 누진세율을 적용.
//!
//! 산출세액 = 과세표준 × 세율 − 누진공제. 지방소득세 = 산출세액 × 10%.
//! 세율표는 2023년 개정(2025 현행)이며 정수 연산으로 정확하다.
//!
//! 주의(verify-or-don't-ship): **과세표준 기준 추정**이다. 각종 세액공제·감면·
//! 가산세·중간예납 등은 반영하지 않는다 — 정확한 신고는 홈택스/세무사에서 확인.
//! 세율·구간은 법 개정 시 바뀌므로 "현행 기준"임을 표면에 밝힌다.

/// 종합소득세 과세표준 구간: (상한원, 세율%, 누진공제원). 2023 개정·2025 현행.
const BRACKETS: &[(i64, i64, i64)] = &[
    (14_000_000, 6, 0),
    (50_000_000, 15, 1_260_000),
    (88_000_000, 24, 5_760_000),
    (150_000_000, 35, 15_440_000),
    (300_000_000, 38, 19_940_000),
    (500_000_000, 40, 25_940_000),
    (1_000_000_000, 42, 35_940_000),
    (i64::MAX, 45, 65_940_000),
];

/// 과세표준 → 산출세액(원). 0 이하면 0. 정수 연산이라 오차 없음.
pub fn income_tax(base: i64) -> i64 {
    if base <= 0 {
        return 0;
    }
    for &(upper, pct, deduct) in BRACKETS {
        if base <= upper {
            return (base * pct / 100 - deduct).max(0);
        }
    }
    0
}

/// 지방소득세 = 산출세액의 10%(원).
pub fn local_tax(income_tax: i64) -> i64 {
    income_tax / 10
}

/// 적용 한계세율(%) — 과세표준이 속한 구간의 세율.
pub fn marginal_rate(base: i64) -> i64 {
    if base <= 0 {
        return 0;
    }
    for &(upper, pct, _) in BRACKETS {
        if base <= upper {
            return pct;
        }
    }
    45
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_examples() {
        // 국세청 누진세율표 검산값.
        assert_eq!(income_tax(30_000_000), 3_240_000); // 324만
        assert_eq!(income_tax(50_000_000), 6_240_000); // 624만
        assert_eq!(income_tax(88_000_000), 15_360_000); // 1536만
        assert_eq!(income_tax(100_000_000), 19_560_000); // 1956만
        assert_eq!(income_tax(150_000_000), 37_060_000); // 3706만
    }

    #[test]
    fn boundary_is_continuous() {
        // 구간 경계에서 위/아래 구간 세액이 같아야 누진공제가 정확.
        assert_eq!(income_tax(14_000_000), 14_000_000 * 6 / 100); // 84만
        assert_eq!(income_tax(50_000_000), 50_000_000 * 15 / 100 - 1_260_000);
        assert_eq!(income_tax(88_000_000), 88_000_000 * 35 / 100 - 15_440_000); // 다음 구간 식과 동일
    }

    #[test]
    fn nonpositive_is_zero() {
        assert_eq!(income_tax(0), 0);
        assert_eq!(income_tax(-1_000), 0);
        assert_eq!(local_tax(0), 0);
    }

    #[test]
    fn local_and_marginal() {
        assert_eq!(local_tax(6_240_000), 624_000); // 산출세액의 10%
        assert_eq!(marginal_rate(30_000_000), 15);
        assert_eq!(marginal_rate(100_000_000), 35);
        assert_eq!(marginal_rate(2_000_000_000), 45);
    }
}
