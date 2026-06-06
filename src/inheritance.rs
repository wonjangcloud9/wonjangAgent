//! 법정 상속분(민법 1000·1009조) — 배우자 5할 가산, 동순위 균분, 1순위 우선.
//!
//! 규칙(오래 안정적이라 검증 가능):
//! - 1순위 직계비속(자녀)이 있으면 자녀+배우자가 상속(직계존속 제외).
//! - 자녀가 없으면 2순위 직계존속(부모)+배우자.
//! - 자녀·부모 모두 없으면 배우자 단독.
//! - 같은 순위는 균분, 배우자만 5할 가산(1.5). half-unit으로 배우자=3, 그 외=2.
//!
//! 주의: 법정 상속분 추정이다. 유언·유류분·기여분·상속포기·대습상속은 미반영.

/// 상속인 한 명의 몫.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heir {
    pub name: String,
    pub num: i64,    // 법정상속분 분자(기약분수)
    pub den: i64,    // 분모
    pub amount: i64, // 금액(원)
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.max(1)
    } else {
        gcd(b, a % b)
    }
}

/// 상속재산 + 상속인 구성 → 각자의 법정상속분·금액. 상속인이 없으면 Err.
pub fn distribute(
    estate: i64,
    spouse: bool,
    children: u32,
    parents: u32,
) -> Result<Vec<Heir>, String> {
    // (이름, half-unit) 목록 구성.
    let mut raw: Vec<(String, i64)> = Vec::new();
    if children > 0 {
        if spouse {
            raw.push(("배우자".into(), 3));
        }
        for i in 1..=children {
            raw.push((format!("자녀{i}"), 2));
        }
    } else if parents > 0 {
        if spouse {
            raw.push(("배우자".into(), 3));
        }
        for i in 1..=parents {
            raw.push((format!("부모{i}"), 2));
        }
    } else if spouse {
        raw.push(("배우자".into(), 2));
    } else {
        return Err(
            "상속인을 지정하세요: --배우자 · --자녀 N · --부모 M (예: 상속 7억 --배우자 --자녀 2)"
                .into(),
        );
    }

    let total: i64 = raw.iter().map(|(_, u)| u).sum();
    let mut heirs: Vec<Heir> = Vec::with_capacity(raw.len());
    let mut assigned = 0i64;
    for (name, u) in &raw {
        let amount = estate.max(0) * u / total;
        assigned += amount;
        let g = gcd(*u, total);
        heirs.push(Heir {
            name: name.clone(),
            num: u / g,
            den: total / g,
            amount,
        });
    }
    // 정수 절사로 남은 원은 첫 상속인에게(합계 = 상속재산).
    if let Some(first) = heirs.first_mut() {
        first.amount += estate.max(0) - assigned;
    }
    Ok(heirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amt(estate: i64, sp: bool, c: u32, p: u32) -> Vec<i64> {
        distribute(estate, sp, c, p)
            .unwrap()
            .iter()
            .map(|h| h.amount)
            .collect()
    }

    #[test]
    fn spouse_and_children() {
        // 7억, 배우자+자녀2 → 배우자 3억, 자녀 2억·2억.
        assert_eq!(
            amt(700_000_000, true, 2, 0),
            vec![300_000_000, 200_000_000, 200_000_000]
        );
        // 분수: 배우자 3/7, 자녀 2/7.
        let h = distribute(700_000_000, true, 2, 0).unwrap();
        assert_eq!((h[0].num, h[0].den), (3, 7));
        assert_eq!((h[1].num, h[1].den), (2, 7));
    }

    #[test]
    fn other_compositions() {
        // 자녀만: 균분.
        assert_eq!(
            amt(500_000_000, false, 2, 0),
            vec![250_000_000, 250_000_000]
        );
        // 배우자+부모(자녀 없음).
        let h = distribute(600_000_000, true, 0, 2).unwrap();
        assert_eq!((h[0].num, h[0].den), (3, 7));
        // 배우자 단독.
        let h = distribute(300_000_000, true, 0, 0).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].amount, 300_000_000);
        // 자녀 있으면 부모 제외.
        assert_eq!(distribute(700_000_000, true, 2, 2).unwrap().len(), 3);
        // 상속인 없음 → Err.
        assert!(distribute(100_000_000, false, 0, 0).is_err());
    }

    #[test]
    fn remainder_kept_total_exact() {
        // 합계가 상속재산과 정확히 일치(나머지는 첫 상속인).
        let h = distribute(1_000_000_000, true, 2, 0).unwrap();
        assert_eq!(h.iter().map(|x| x.amount).sum::<i64>(), 1_000_000_000);
    }
}
