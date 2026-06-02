//! 수면 시간 계산 — 90분 수면 주기로 개운한 취침/기상 시각을 추천한다.
//!
//! 잠드는 데 약 14분이 걸린다고 보고, 90분 주기의 끝에 깨면 개운하다는 통념을
//! 따른다(수면 주기 계산기). 순수 시간 계산이라 키·저장이 없다.

use anyhow::{anyhow, Result};
use chrono::{Duration, NaiveTime, Timelike};

const CYCLE_MIN: i64 = 90; // 수면 주기
const FALL_ASLEEP_MIN: i64 = 14; // 잠드는 시간

fn fmt(t: NaiveTime) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}

/// 기상 시각이 정해졌을 때 추천 취침 시각들(주기 6→3, 잘 잘수록 먼저).
pub fn bedtimes_for_wake(wake: NaiveTime) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for cycles in [6i64, 5, 4, 3] {
        let back = Duration::minutes(cycles * CYCLE_MIN + FALL_ASLEEP_MIN);
        let bed = wake - back;
        out.push((cycles as u32, fmt(bed)));
    }
    out
}

/// 지금(또는 주어진 취침 시각) 잘 때 추천 기상 시각들(주기 3→6).
pub fn waketimes_for_bed(bed: NaiveTime) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let asleep = bed + Duration::minutes(FALL_ASLEEP_MIN);
    for cycles in [3i64, 4, 5, 6] {
        let wake = asleep + Duration::minutes(cycles * CYCLE_MIN);
        out.push((cycles as u32, fmt(wake)));
    }
    out
}

/// "HH:MM" 파싱.
pub fn parse_time(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M")
        .map_err(|_| anyhow!("시각은 HH:MM 형식으로 입력하세요 (예: 07:00)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn bedtimes_count_and_wrap() {
        let b = bedtimes_for_wake(t(7, 0));
        assert_eq!(b.len(), 4);
        // 6주기: 07:00 - (540+14)분 = 07:00 - 9시간14분 = 21:46.
        assert_eq!(b[0], (6, "21:46".to_string()));
    }

    #[test]
    fn waketimes_from_bed() {
        let w = waketimes_for_bed(t(23, 0));
        assert_eq!(w.len(), 4);
        // 3주기: 23:00 + 14분 + 270분 = 23:14 + 4h30 = 03:44.
        assert_eq!(w[0], (3, "03:44".to_string()));
    }
}
