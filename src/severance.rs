//! 퇴직금 계산 — 근로기준법상 법정 퇴직금 추정(순수 공식, 키 불필요).
//!
//! 퇴직금 = 1일 평균임금 × 30일 × (재직일수 / 365). 1일 평균임금은 직전 3개월
//! 임금 ÷ 그 기간 일수인데, 월급이 일정하다고 보면 ≈ 월급 × 12 / 365(연봉/365)이다.
//! 평균임금엔 상여·연차수당이 더해지므로 실제 퇴직금은 이 추정보다 많을 수 있다.
//! 요율표가 아니라 법 공식이라 해마다 바뀌지 않는다.

/// 한 달(평균) 일수 — 근속 개월을 일수로 환산할 때.
pub const DAYS_PER_MONTH: f64 = 365.0 / 12.0;

/// 월 평균임금(만원)과 재직일수로 법정 퇴직금(만원)을 추정한다.
pub fn severance_manwon(monthly_manwon: f64, days: i64) -> f64 {
    let daily_avg = monthly_manwon * 12.0 / 365.0; // 1일 평균임금(만원/일)
    daily_avg * 30.0 * (days as f64 / 365.0)
}

/// 근속 년·개월을 재직일수로(평균 한 달 = 365/12일).
pub fn service_days(years: u32, months: u32) -> i64 {
    let total_months = years as f64 * 12.0 + months as f64;
    (total_months * DAYS_PER_MONTH).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_year_is_about_one_month_salary() {
        // 1년(365일) → 360×W/365 ≈ 0.986달치. W=300 → 295.89만원.
        let s = severance_manwon(300.0, 365);
        assert!((s - 295.89).abs() < 0.5, "{s}");
    }

    #[test]
    fn scales_linearly_with_days() {
        // 재직일수에 정확히 비례(3년 = 1년의 3배).
        let one = severance_manwon(300.0, 365);
        let three = severance_manwon(300.0, 1095);
        assert!((three - one * 3.0).abs() < 0.001);
    }

    #[test]
    fn matches_labor_ministry_within_one_percent() {
        // 고용노동부 예시(월급 300만·3년) 8,804,348원 ≈ 880.4만원. 근사식은 ~0.8% 이내.
        let s = severance_manwon(300.0, service_days(3, 0));
        assert!((s - 880.4).abs() / 880.4 < 0.01, "{s}");
    }

    #[test]
    fn service_days_3years() {
        assert_eq!(service_days(3, 0), 1095); // 36 × 30.4167 ≈ 1095
        assert_eq!(service_days(0, 12), 365);
    }
}
