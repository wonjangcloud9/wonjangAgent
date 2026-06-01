//! 더치페이(n빵) 정산 — 모임·회식 비용을 인원수로 나눈다.
//!
//! 1인당 금액을 깔끔한 단위로 올림하여(기본 100원) 모두 같은 금액을 내고,
//! 총무에게 남는 거스름돈까지 알려준다. 순수 계산이라 키가 필요 없다.

/// 더치페이 정산 결과.
#[derive(Debug, Clone)]
pub struct Split {
    pub total: i64,      // 총액
    pub people: i64,     // 인원수
    pub per_person: i64, // 1인당 낼 금액(올림)
    pub collected: i64,  // 걷히는 총액(1인당 × 인원)
    pub leftover: i64,   // 총무에게 남는 거스름(걷힌 금액 − 총액)
    pub exact: f64,      // 올림 전 정확한 1인당 금액
}

/// 총액을 인원수로 나눠 정산한다. `unit`은 올림 단위(예: 100원).
pub fn split(total: i64, people: i64, unit: i64) -> Option<Split> {
    if people <= 0 || total < 0 {
        return None;
    }
    let unit = unit.max(1);
    let exact = total as f64 / people as f64;
    // unit 단위로 올림(div_ceil은 아직 unstable이라 수동 계산).
    let base = exact.ceil() as i64;
    let per_person = ((base + unit - 1) / unit) * unit;
    let collected = per_person * people;
    Some(Split {
        total,
        people,
        per_person,
        collected,
        leftover: collected - total,
        exact,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_split_no_leftover() {
        // 40,000원 4명 → 1인당 10,000원, 거스름 0.
        let s = split(40_000, 4, 100).unwrap();
        assert_eq!(s.per_person, 10_000);
        assert_eq!(s.leftover, 0);
    }

    #[test]
    fn rounds_up_to_unit() {
        // 50,000원 3명 → 정확히 16,666.67 → 100원 올림 16,700.
        let s = split(50_000, 3, 100).unwrap();
        assert_eq!(s.per_person, 16_700);
        assert_eq!(s.collected, 50_100);
        assert_eq!(s.leftover, 100);
    }

    #[test]
    fn thousand_unit_rounding() {
        // 50,000원 3명 → 1,000원 단위 올림 17,000.
        let s = split(50_000, 3, 1_000).unwrap();
        assert_eq!(s.per_person, 17_000);
        assert_eq!(s.leftover, 1_000);
    }

    #[test]
    fn invalid_inputs() {
        assert!(split(10_000, 0, 100).is_none());
        assert!(split(-1, 3, 100).is_none());
    }
}
