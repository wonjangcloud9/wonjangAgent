//! 연차 유급휴가 일수 — 근로기준법 제60조. 순수 공식이라 키도, 매년 바뀌는 요율도 없다.
//!
//! - 입사 1년 미만: 1개월 개근당 1일(최대 11일).
//! - 1년 이상(80% 이상 출근): 15일.
//! - 3년 이상: 15일 + 매 2년마다 1일 가산, 한도 25일.

/// 근속 년·개월 시점에 발생하는 연차 일수.
pub fn annual_leave_days(years: u32, months: u32) -> u32 {
    if years == 0 {
        months.min(11) // 1년 미만: 1개월 개근당 1일, 최대 11
    } else {
        (15 + (years - 1) / 2).min(25) // 3년차부터 매 2년 +1, 상한 25
    }
}

/// 다음으로 연차가 늘어나는 `(근속년수, 그때 일수)`. 이미 상한(25일)이면 None.
pub fn next_increase(years: u32) -> Option<(u32, u32)> {
    if annual_leave_days(years.max(1), 0) >= 25 {
        return None;
    }
    // 가산은 3,5,7,… 홀수 해부터. 현재 이후 첫 가산 해.
    let next = if years < 3 {
        3
    } else if years.is_multiple_of(2) {
        years + 1
    } else {
        years + 2
    };
    Some((next, annual_leave_days(next, 0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statutory_leave_days() {
        assert_eq!(annual_leave_days(0, 6), 6); // 6개월 → 6일
        assert_eq!(annual_leave_days(0, 11), 11);
        assert_eq!(annual_leave_days(1, 0), 15);
        assert_eq!(annual_leave_days(2, 0), 15);
        assert_eq!(annual_leave_days(3, 0), 16); // 3년차 +1
        assert_eq!(annual_leave_days(5, 0), 17);
        assert_eq!(annual_leave_days(10, 0), 19);
        assert_eq!(annual_leave_days(21, 0), 25); // 상한 도달
        assert_eq!(annual_leave_days(30, 0), 25); // 상한 유지
    }

    #[test]
    fn next_increase_points() {
        assert_eq!(next_increase(1), Some((3, 16)));
        assert_eq!(next_increase(2), Some((3, 16)));
        assert_eq!(next_increase(4), Some((5, 17)));
        assert_eq!(next_increase(20), Some((21, 25)));
        assert_eq!(next_increase(21), None); // 이미 상한
    }
}
