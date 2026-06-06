//! 법정 최고금리 체크 — 사채·대출 이자가 이자제한법(연 20%) 한도를 넘는지.
//!
//! 사채업자가 "월 5만"처럼 단기 금액으로 높은 연이율을 숨기는 걸, 연 환산해 잡아준다.
//! 최고이자율 연 20%는 이자제한법 시행령·대부업법(2021.7.7~)으로 안정적이라 검증 가능.
//!
//! 주의: 단리 연환산 기준의 추정이다(복리·수수료·선이자 등은 별도). 불법 의심 시 금융감독원·법률 상담.

/// 법정 최고이자율(연, %). 이자제한법/대부업법, 2021.7~.
pub const LEGAL_MAX_PERCENT: f64 = 20.0;

/// 최고금리 체크 결과.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Check {
    pub annual_rate: f64,  // 환산 연이율(%)
    pub legal: bool,       // 20% 이하인가
    pub max_interest: i64, // 같은 기간 20% 기준 최대 이자
}

/// 원금·이자총액·기간(개월) → 연이율·합법 여부. 원금/기간이 0 이하면 0%.
pub fn check(principal: i64, interest: i64, months: f64) -> Check {
    let annual = if principal > 0 && months > 0.0 {
        (interest as f64 / principal as f64) / (months / 12.0) * 100.0
    } else {
        0.0
    };
    let max_interest = if principal > 0 && months > 0.0 {
        (principal as f64 * LEGAL_MAX_PERCENT / 100.0 * (months / 12.0)) as i64
    } else {
        0
    };
    Check {
        annual_rate: annual,
        legal: annual <= LEGAL_MAX_PERCENT + 1e-9,
        max_interest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn illegal_monthly_disguise() {
        // 원금 100만, 월 이자 5만(1개월) → 연 60% → 불법.
        let c = check(1_000_000, 50_000, 1.0);
        assert!((c.annual_rate - 60.0).abs() < 0.01);
        assert!(!c.legal);
        assert_eq!(c.max_interest, 16_666);
    }

    #[test]
    fn legal_boundary_20_percent() {
        // 연 20%는 합법(한도 이하).
        let c = check(1_000_000, 200_000, 12.0);
        assert!((c.annual_rate - 20.0).abs() < 0.01);
        assert!(c.legal);
        // 15%도 합법.
        assert!(check(1_000_000, 150_000, 12.0).legal);
    }

    #[test]
    fn zero_inputs_safe() {
        let c = check(0, 50_000, 1.0);
        assert_eq!(c.annual_rate, 0.0);
        assert!(c.legal);
    }
}
