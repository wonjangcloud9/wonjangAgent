//! 증여세 추정 — 증여재산공제(10년 합산) → 과세표준 → 누진세율 → 신고세액공제(3%).
//!
//! 세율(상속·증여 공통)·공제는 수년째 안정적이라 검증 가능하다. 정수 연산으로 정확.
//!
//! 주의(verify-or-don't-ship): **기본 증여 추정**이다. 10년 내 합산증여·세대생략 할증
//! (직계비속 외 손주 등 30~40%)·재산 종류별 평가·가업승계 특례 등은 반영하지 않는다.
//! 정확한 신고는 홈택스/세무사에서. 세율·공제는 법 개정 시 바뀌므로 "현행 기준" 표기.

/// 증여자–수증자 관계(증여재산공제 결정, 10년 합산 기준).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Spouse,        // 배우자 6억
    AdultChild,    // 성년 직계비속 5천만
    MinorChild,    // 미성년 직계비속 2천만
    Ascendant,     // 직계존속(부모·조부모) 5천만
    OtherRelative, // 기타 친족(6촌 이내 혈족·4촌 이내 인척) 1천만
    Other,         // 타인 0
}

impl Relation {
    /// 증여재산공제액(원).
    pub fn deduction(self) -> i64 {
        match self {
            Relation::Spouse => 600_000_000,
            Relation::AdultChild => 50_000_000,
            Relation::MinorChild => 20_000_000,
            Relation::Ascendant => 50_000_000,
            Relation::OtherRelative => 10_000_000,
            Relation::Other => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Relation::Spouse => "배우자",
            Relation::AdultChild => "성년 자녀",
            Relation::MinorChild => "미성년 자녀",
            Relation::Ascendant => "직계존속(부모·조부모)",
            Relation::OtherRelative => "기타 친족",
            Relation::Other => "타인",
        }
    }
}

/// `"자녀"`·`"배우자"`·`"미성년"` 같은 입력을 관계로 파싱한다(기본 성년 자녀).
pub fn parse_relation(s: &str) -> Result<Relation, String> {
    match s.trim() {
        "배우자" | "부부" => Ok(Relation::Spouse),
        "자녀" | "성년자녀" | "성인자녀" | "성년" | "자식" | "아들" | "딸" => Ok(Relation::AdultChild),
        "미성년" | "미성년자녀" | "미성년자" => Ok(Relation::MinorChild),
        "부모" | "직계존속" | "조부모" | "할아버지" | "할머니" | "엄마" | "아빠" => {
            Ok(Relation::Ascendant)
        }
        "기타" | "친족" | "기타친족" | "형제" | "남매" | "친척" => Ok(Relation::OtherRelative),
        "타인" | "제3자" | "남" => Ok(Relation::Other),
        other => Err(format!(
            "관계는 배우자·자녀·미성년·부모·기타·타인 중 하나예요 (예: 증여세 5억 자녀). 입력: '{other}'"
        )),
    }
}

/// 증여세 누진세율 구간: (상한원, 세율%, 누진공제원). 상속·증여 공통, 현행.
const BRACKETS: &[(i64, i64, i64)] = &[
    (100_000_000, 10, 0),
    (500_000_000, 20, 10_000_000),
    (1_000_000_000, 30, 60_000_000),
    (3_000_000_000, 40, 160_000_000),
    (i64::MAX, 50, 460_000_000),
];

/// 과세표준 → 산출세액(원). 0 이하면 0.
pub fn tax_on_base(base: i64) -> i64 {
    if base <= 0 {
        return 0;
    }
    for &(upper, pct, deduct) in BRACKETS {
        if base <= upper {
            return (base * pct / 100 - deduct).max(0);
        }
    }
    0
}

/// 증여세 계산 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GiftTax {
    pub deduction: i64,     // 적용 증여재산공제
    pub base: i64,          // 과세표준
    pub calculated: i64,    // 산출세액
    pub filing_credit: i64, // 신고세액공제(3%)
    pub payable: i64,       // 납부세액
}

/// 증여액 + 관계 → 증여세. 공제는 증여액을 넘지 않는다.
pub fn compute(amount: i64, rel: Relation) -> GiftTax {
    let deduction = rel.deduction().min(amount.max(0));
    let base = (amount - deduction).max(0);
    let calculated = tax_on_base(base);
    let filing_credit = calculated * 3 / 100; // 자진신고 3% 공제
    GiftTax {
        deduction,
        base,
        calculated,
        filing_credit,
        payable: calculated - filing_credit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_examples() {
        // 성년 자녀 1억 → 과표 5천만 → 산출 500만 → 납부 485만.
        let g = compute(100_000_000, Relation::AdultChild);
        assert_eq!(g.base, 50_000_000);
        assert_eq!(g.calculated, 5_000_000);
        assert_eq!(g.payable, 4_850_000);
        // 성년 자녀 5억 → 과표 4.5억 → 산출 8천만 → 납부 7,760만.
        let g = compute(500_000_000, Relation::AdultChild);
        assert_eq!(g.calculated, 80_000_000);
        assert_eq!(g.payable, 77_600_000);
        // 배우자 6억 → 전액 공제 → 0.
        let g = compute(600_000_000, Relation::Spouse);
        assert_eq!(g.base, 0);
        assert_eq!(g.payable, 0);
    }

    #[test]
    fn boundary_continuous() {
        assert_eq!(tax_on_base(100_000_000), 10_000_000); // 10%·20% 경계 동일
        assert_eq!(tax_on_base(500_000_000), 90_000_000); // 20%·30% 경계 동일
        assert_eq!(tax_on_base(1_000_000_000), 240_000_000);
    }

    #[test]
    fn deduction_capped_and_relations() {
        // 공제가 증여액보다 크면 증여액까지만(과표 0).
        let g = compute(30_000_000, Relation::AdultChild);
        assert_eq!(g.base, 0);
        assert_eq!(g.payable, 0);
        assert_eq!(parse_relation("자녀"), Ok(Relation::AdultChild));
        assert_eq!(parse_relation("배우자"), Ok(Relation::Spouse));
        assert_eq!(parse_relation("미성년"), Ok(Relation::MinorChild));
        assert!(parse_relation("회사").is_err());
    }
}
