//! 시급·주휴수당 계산 — 알바·시간제 근로자의 주급/월급을 추정한다.
//!
//! 근로기준법상 1주 소정근로시간이 15시간 이상이면 주휴수당이 발생한다.
//! 주휴수당 = (1주 소정근로시간 ÷ 40) × 8 × 시급 (상한 8시간분). 순수 계산.

/// 2025년 법정 최저시급(원).
pub const MIN_WAGE_2025: f64 = 10_030.0;

/// 주휴수당 발생 최소 주당 근로시간.
const HOLIDAY_THRESHOLD: f64 = 15.0;

/// 한 달 평균 주 수(365 ÷ 7 ÷ 12).
const WEEKS_PER_MONTH: f64 = 365.0 / 7.0 / 12.0;

/// 임금 계산 결과.
pub struct Wage {
    pub hourly: f64,       // 시급
    pub weekly_hours: f64, // 주당 근로시간
    pub base_weekly: f64,  // 주 기본급(시급×시간)
    pub holiday_pay: f64,  // 주휴수당(주당)
    pub weekly_total: f64, // 주급 합계
    pub monthly: f64,      // 월 환산(주급×4.345)
    pub below_min: bool,   // 최저시급 미만 여부
}

/// 시급과 주당 근로시간으로 임금을 계산한다.
pub fn calc(hourly: f64, weekly_hours: f64) -> Wage {
    let base_weekly = hourly * weekly_hours;
    let holiday_pay = if weekly_hours >= HOLIDAY_THRESHOLD {
        // 주 40시간 비례, 상한 8시간분.
        (weekly_hours.min(40.0) / 40.0) * 8.0 * hourly
    } else {
        0.0
    };
    let weekly_total = base_weekly + holiday_pay;
    Wage {
        hourly,
        weekly_hours,
        base_weekly,
        holiday_pay,
        weekly_total,
        monthly: weekly_total * WEEKS_PER_MONTH,
        below_min: hourly < MIN_WAGE_2025,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_time_minimum_wage() {
        // 최저시급 주 40시간 → 주휴 8시간분, 월 약 209만.
        let w = calc(MIN_WAGE_2025, 40.0);
        assert!((w.holiday_pay - 80_240.0).abs() < 1.0);
        assert!((w.monthly - 2_090_000.0).abs() < 5_000.0);
        assert!(!w.below_min);
    }

    #[test]
    fn part_time_holiday_proportional() {
        // 주 20시간 → 주휴 4시간분.
        let w = calc(10_000.0, 20.0);
        assert!((w.holiday_pay - 40_000.0).abs() < 1.0);
    }

    #[test]
    fn under_15h_no_holiday_pay() {
        let w = calc(10_000.0, 14.0);
        assert_eq!(w.holiday_pay, 0.0);
    }

    #[test]
    fn detects_below_minimum() {
        assert!(calc(9_000.0, 20.0).below_min);
    }
}
