//! 대출 상환 계산.
//!
//! 원리금균등상환(매달 같은 금액)과 원금균등상환(매달 같은 원금) 두 방식의
//! 월 상환액·총이자·총상환액을 계산한다. 순수 금융 공식이라 키가 필요 없다.

/// 원리금균등상환 명세.
#[derive(Debug, Clone)]
pub struct EqualPayment {
    pub monthly: f64,        // 매달 상환액(일정)
    pub total_payment: f64,  // 총 상환액
    pub total_interest: f64, // 총 이자
}

/// 원금균등상환 명세.
#[derive(Debug, Clone)]
pub struct EqualPrincipal {
    pub first_month: f64,    // 첫 달 상환액(가장 큼)
    pub last_month: f64,     // 마지막 달 상환액(가장 작음)
    pub total_payment: f64,  // 총 상환액
    pub total_interest: f64, // 총 이자
}

/// 월 이자율(연이율 %를 12로 나눈 소수).
fn monthly_rate(annual_pct: f64) -> f64 {
    annual_pct / 100.0 / 12.0
}

/// 원리금균등상환: 매달 같은 금액을 갚는다.
///
/// 월 상환액 = P·r·(1+r)^n / ((1+r)^n − 1). 무이자(r=0)면 P/n.
pub fn equal_payment(principal: f64, annual_pct: f64, months: u32) -> EqualPayment {
    let n = months.max(1) as f64;
    let r = monthly_rate(annual_pct);
    let monthly = if r == 0.0 {
        principal / n
    } else {
        let pow = (1.0 + r).powf(n);
        principal * r * pow / (pow - 1.0)
    };
    let total_payment = monthly * n;
    EqualPayment {
        monthly,
        total_payment,
        total_interest: total_payment - principal,
    }
}

/// 원금균등상환: 매달 원금은 같고 이자는 잔액에 따라 줄어든다.
pub fn equal_principal(principal: f64, annual_pct: f64, months: u32) -> EqualPrincipal {
    let n = months.max(1);
    let r = monthly_rate(annual_pct);
    let principal_part = principal / n as f64;
    let mut balance = principal;
    let mut total_interest = 0.0;
    let mut first = 0.0;
    let mut last = 0.0;
    for i in 0..n {
        let interest = balance * r;
        let payment = principal_part + interest;
        if i == 0 {
            first = payment;
        }
        if i == n - 1 {
            last = payment;
        }
        total_interest += interest;
        balance -= principal_part;
    }
    EqualPrincipal {
        first_month: first,
        last_month: last,
        total_payment: principal + total_interest,
        total_interest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_payment_1000man_6pct_12m() {
        // 원금 1천만, 연 6%, 12개월 → 월 약 860,664원.
        let p = equal_payment(10_000_000.0, 6.0, 12);
        assert!((p.monthly - 860_664.0).abs() < 5.0, "monthly={}", p.monthly);
        // 총이자 약 32.8만.
        assert!((p.total_interest - 327_968.0).abs() < 50.0);
    }

    #[test]
    fn zero_interest_splits_evenly() {
        let p = equal_payment(1_200_000.0, 0.0, 12);
        assert_eq!(p.monthly, 100_000.0);
        assert_eq!(p.total_interest, 0.0);
    }

    #[test]
    fn equal_principal_first_bigger_than_last() {
        let p = equal_principal(10_000_000.0, 6.0, 12);
        assert!(p.first_month > p.last_month);
        // 원금균등 총이자는 원리금균등보다 적다.
        let ep = equal_payment(10_000_000.0, 6.0, 12);
        assert!(p.total_interest < ep.total_interest);
    }
}
