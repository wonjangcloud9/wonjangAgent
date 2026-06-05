//! 야근·휴일 수당 — 근로기준법 가산율. 순수 공식, 키 불필요. 요율이 아니라 법이라 staleness 없음.
//!
//! - 연장근로(법정근로 초과): 통상임금 × 1.5
//! - 야간근로(22:00~06:00): 통상임금 × 0.5 가산(연장·휴일과 겹치면 더해짐)
//! - 휴일근로(8시간 이내): 통상임금 × 1.5

/// 각 수당(원). 통상시급(원)과 시간 유형별 시간으로.
#[derive(Debug, Clone, Copy)]
pub struct Allowances {
    pub overtime: f64, // 연장수당
    pub night: f64,    // 야간 가산
    pub holiday: f64,  // 휴일수당
}

impl Allowances {
    pub fn total(&self) -> f64 {
        self.overtime + self.night + self.holiday
    }
}

/// 통상시급·시간으로 수당을 계산한다.
pub fn calc(hourly: f64, overtime_h: f64, night_h: f64, holiday_h: f64) -> Allowances {
    Allowances {
        overtime: hourly * 1.5 * overtime_h,
        night: hourly * 0.5 * night_h,
        holiday: hourly * 1.5 * holiday_h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowances_by_type() {
        // 통상시급 12,000원 기준.
        let a = calc(12_000.0, 3.0, 2.0, 8.0);
        assert_eq!(a.overtime, 54_000.0); // 12000×1.5×3
        assert_eq!(a.night, 12_000.0); // 12000×0.5×2
        assert_eq!(a.holiday, 144_000.0); // 12000×1.5×8
        assert_eq!(a.total(), 210_000.0);
    }

    #[test]
    fn only_overtime() {
        let a = calc(10_030.0, 2.0, 0.0, 0.0);
        assert_eq!(a.overtime, 30_090.0); // 최저시급 2시간 연장
        assert_eq!(a.total(), 30_090.0);
    }
}
