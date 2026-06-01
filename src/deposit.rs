//! 예·적금 만기 이자 계산.
//!
//! 정기예금(목돈 예치, 단리)과 정기적금(매달 적립, 단리)의 만기 수령액을
//! 계산한다. 이자에는 이자소득세 15.4%(소득세 14% + 지방소득세 1.4%)를
//! 적용한다. 순수 금융 공식이라 키가 필요 없다.

/// 이자소득세율(소득세 14% + 지방소득세 1.4%).
pub const TAX_RATE: f64 = 0.154;

/// 예·적금 결과 명세.
#[derive(Debug, Clone)]
pub struct Maturity {
    pub principal: f64,         // 원금 합계
    pub interest_pretax: f64,   // 세전 이자
    pub tax: f64,               // 이자소득세
    pub interest_aftertax: f64, // 세후 이자
    pub total: f64,             // 만기 수령액(원금 + 세후 이자)
}

fn finish(principal: f64, interest_pretax: f64) -> Maturity {
    let tax = interest_pretax * TAX_RATE;
    let interest_aftertax = interest_pretax - tax;
    Maturity {
        principal,
        interest_pretax,
        tax,
        interest_aftertax,
        total: principal + interest_aftertax,
    }
}

/// 정기예금(목돈 예치, 단리): 이자 = 원금 × 연이율 × 개월/12.
pub fn time_deposit(principal: f64, annual_pct: f64, months: u32) -> Maturity {
    let r = annual_pct / 100.0;
    let interest = principal * r * (months as f64 / 12.0);
    finish(principal, interest)
}

/// 정기적금(매달 적립, 단리): 이자 = 월납입 × (연이율/12) × n(n+1)/2.
///
/// 첫 달 납입은 n개월, 마지막 달은 1개월치 이자가 붙는 누적 구조.
pub fn installment(monthly: f64, annual_pct: f64, months: u32) -> Maturity {
    let n = months as f64;
    let r = annual_pct / 100.0 / 12.0;
    let interest = monthly * r * (n * (n + 1.0) / 2.0);
    finish(monthly * n, interest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_deposit_1000man_6pct_12m() {
        // 원금 1천만, 연 6%, 12개월 → 세전이자 60만, 세후 507,600.
        let m = time_deposit(10_000_000.0, 6.0, 12);
        assert_eq!(m.interest_pretax, 600_000.0);
        assert!((m.interest_aftertax - 507_600.0).abs() < 1.0);
        assert!((m.total - 10_507_600.0).abs() < 1.0);
    }

    #[test]
    fn installment_10man_6pct_12m() {
        // 월 10만, 연 6%, 12개월 → 세전이자 39,000, 원금 120만.
        let m = installment(100_000.0, 6.0, 12);
        assert_eq!(m.principal, 1_200_000.0);
        assert!((m.interest_pretax - 39_000.0).abs() < 1.0);
        // 세후 = 39,000 × 0.846 = 32,994.
        assert!((m.interest_aftertax - 32_994.0).abs() < 1.0);
    }

    #[test]
    fn tax_is_154_percent() {
        let m = time_deposit(10_000_000.0, 10.0, 12);
        assert!((m.tax / m.interest_pretax - 0.154).abs() < 1e-9);
    }
}
