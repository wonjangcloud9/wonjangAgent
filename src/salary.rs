//! 연봉 실수령액 계산.
//!
//! 연봉(세전)을 입력하면 4대 보험과 근로소득세를 공제한 월 실수령액을
//! 추정한다. 보험 요율은 2025년 기준(근로자 부담분). 소득세는 연간
//! 근로소득세 산출 과정을 그대로 계산한 뒤 12로 나눈 추정치다.
//!
//! 기본 가정: 1인 가구(본인 기본공제만), 비과세 식대 월 20만 원.

// 2025년 근로자 부담 4대 보험 요율.
const NP_RATE: f64 = 0.045; // 국민연금 4.5%
const NP_BASE_CAP: f64 = 6_370_000.0; // 국민연금 기준소득월액 상한
const HI_RATE: f64 = 0.03545; // 건강보험 3.545%
const LTC_RATE: f64 = 0.1295; // 장기요양 = 건강보험료의 12.95%
const EI_RATE: f64 = 0.009; // 고용보험 0.9%

const TAX_FREE_MONTHLY: f64 = 200_000.0; // 비과세 식대(월)
const BASIC_DEDUCTION: f64 = 1_500_000.0; // 본인 기본공제(연)

/// 월 단위 공제 내역과 실수령액.
#[derive(Debug, Clone)]
pub struct Payslip {
    pub gross_monthly: f64,    // 월 세전(연봉/12)
    pub national_pension: f64, // 국민연금
    pub health: f64,           // 건강보험
    pub long_term_care: f64,   // 장기요양
    pub employment: f64,       // 고용보험
    pub income_tax: f64,       // 근로소득세(월)
    pub local_tax: f64,        // 지방소득세(월)
}

impl Payslip {
    /// 월 공제 합계.
    pub fn total_deduction(&self) -> f64 {
        self.national_pension
            + self.health
            + self.long_term_care
            + self.employment
            + self.income_tax
            + self.local_tax
    }

    /// 월 실수령액.
    pub fn net_monthly(&self) -> f64 {
        self.gross_monthly - self.total_deduction()
    }
}

/// 근로소득공제(연).
fn earned_income_deduction(gross: f64) -> f64 {
    match gross {
        g if g <= 5_000_000.0 => g * 0.7,
        g if g <= 15_000_000.0 => 3_500_000.0 + (g - 5_000_000.0) * 0.4,
        g if g <= 45_000_000.0 => 7_500_000.0 + (g - 15_000_000.0) * 0.15,
        g if g <= 100_000_000.0 => 12_000_000.0 + (g - 45_000_000.0) * 0.05,
        g => 14_750_000.0 + (g - 100_000_000.0) * 0.02,
    }
}

/// 종합소득 과세표준에 대한 산출세액(2025년 누진세율).
fn progressive_tax(base: f64) -> f64 {
    match base {
        b if b <= 14_000_000.0 => b * 0.06,
        b if b <= 50_000_000.0 => 840_000.0 + (b - 14_000_000.0) * 0.15,
        b if b <= 88_000_000.0 => 6_240_000.0 + (b - 50_000_000.0) * 0.24,
        b if b <= 150_000_000.0 => 15_360_000.0 + (b - 88_000_000.0) * 0.35,
        b if b <= 300_000_000.0 => 37_060_000.0 + (b - 150_000_000.0) * 0.38,
        b if b <= 500_000_000.0 => 94_060_000.0 + (b - 300_000_000.0) * 0.40,
        b if b <= 1_000_000_000.0 => 174_060_000.0 + (b - 500_000_000.0) * 0.42,
        b => 384_060_000.0 + (b - 1_000_000_000.0) * 0.45,
    }
}

/// 근로소득세액공제(연). 산출세액과 총급여 기준 한도를 반영.
fn earned_income_tax_credit(calculated_tax: f64, gross: f64) -> f64 {
    let credit = if calculated_tax <= 1_300_000.0 {
        calculated_tax * 0.55
    } else {
        715_000.0 + (calculated_tax - 1_300_000.0) * 0.30
    };
    // 총급여 구간별 한도.
    let cap = if gross <= 33_000_000.0 {
        740_000.0
    } else if gross <= 70_000_000.0 {
        (740_000.0 - (gross - 33_000_000.0) * 0.008).max(660_000.0)
    } else if gross <= 120_000_000.0 {
        (660_000.0 - (gross - 70_000_000.0) * 0.5).max(500_000.0)
    } else {
        (500_000.0 - (gross - 120_000_000.0) * 0.5).max(200_000.0)
    };
    credit.min(cap)
}

/// 연봉(세전, 원)으로 월 실수령 명세를 계산한다.
pub fn from_annual(annual: f64) -> Payslip {
    let gross_monthly = annual / 12.0;
    let taxable_monthly = (gross_monthly - TAX_FREE_MONTHLY).max(0.0);

    // 4대 보험(월).
    let np = taxable_monthly.min(NP_BASE_CAP) * NP_RATE;
    let hi = taxable_monthly * HI_RATE;
    let ltc = hi * LTC_RATE;
    let ei = taxable_monthly * EI_RATE;

    // 연간 근로소득세 산출.
    let gross_for_tax = (annual - TAX_FREE_MONTHLY * 12.0).max(0.0); // 총급여(비과세 제외)
    let income_amount = gross_for_tax - earned_income_deduction(gross_for_tax); // 근로소득금액
                                                                                // 종합소득공제: 기본공제 + 연금보험료공제 + 보험료 특별소득공제.
    let deductions = BASIC_DEDUCTION + (np + hi + ltc + ei) * 12.0;
    let tax_base = (income_amount - deductions).max(0.0); // 과세표준
    let calculated = progressive_tax(tax_base); // 산출세액
    let credit = earned_income_tax_credit(calculated, gross_for_tax);
    let determined = (calculated - credit).max(0.0); // 결정세액(연)
    let income_tax = determined / 12.0; // 월 소득세
    let local_tax = income_tax * 0.10; // 지방소득세 10%

    Payslip {
        gross_monthly,
        national_pension: np.round(),
        health: hi.round(),
        long_term_care: ltc.round(),
        employment: ei.round(),
        income_tax: income_tax.round(),
        local_tax: local_tax.round(),
    }
}

/// 월급(세전, 원)으로 월 실수령 명세를 계산한다(연봉=월급×12로 환산).
/// 많은 사람이 연봉이 아니라 '월급'으로 떠올리기 때문에 입력 편의를 준다.
pub fn from_monthly(monthly: f64) -> Payslip {
    from_annual(monthly * 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monthly_equals_annual_div_12() {
        // 월급 300만 입력 = 연봉 3600만 입력과 같아야(환산만 다름).
        let m = from_monthly(3_000_000.0);
        let a = from_annual(36_000_000.0);
        assert_eq!(m.gross_monthly, a.gross_monthly);
        assert_eq!(m.national_pension, a.national_pension);
        assert_eq!(m.net_monthly(), a.net_monthly());
    }

    #[test]
    fn four_insurances_for_3600() {
        // 연봉 3600만 → 월 세전 300만, 과세 280만.
        let p = from_annual(36_000_000.0);
        assert_eq!(p.gross_monthly, 3_000_000.0);
        assert_eq!(p.national_pension, 126_000.0); // 280만×4.5%
        assert_eq!(p.health, 99_260.0); // 280만×3.545%
        assert_eq!(p.long_term_care, 12_854.0); // 건강×12.95%
        assert_eq!(p.employment, 25_200.0); // 280만×0.9%
    }

    #[test]
    fn net_is_reasonable_for_3600() {
        let p = from_annual(36_000_000.0);
        let net = p.net_monthly();
        // 실수령은 세전보다 작고, 통상 265만 안팎.
        assert!(net < p.gross_monthly);
        assert!((2_600_000.0..2_720_000.0).contains(&net), "net={net}");
    }

    #[test]
    fn pension_cap_applies_on_high_salary() {
        // 연봉 1.2억 → 과세 약 980만, 상한(637만) 적용.
        let p = from_annual(120_000_000.0);
        assert_eq!(p.national_pension.round(), (NP_BASE_CAP * NP_RATE).round());
    }

    #[test]
    fn higher_salary_higher_tax() {
        let low = from_annual(30_000_000.0);
        let high = from_annual(80_000_000.0);
        assert!(high.income_tax > low.income_tax);
    }
}
