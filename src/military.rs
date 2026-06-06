//! 전역일 계산기 — 한국 남성·가족이 가장 집착하는 D-day.
//!
//! 입대일 + 군별 복무기간으로 전역일·전역 D-day·복무 진행률·현재 계급·다음
//! 진급까지를 계산한다. 복무기간(육군/해병 18·해군 20·공군 21·사회복무 21개월)은
//! 2020년 단계적 단축 완료 후 현행이며, 병 계급 진급 경계(입대 후 2·8·14개월)는
//! 군인사법 병 진급 최저복무기간(이병→일병 2, 일병→상병 6, 상병→병장 6개월)으로
//! 전군(육·해·공·해병) 동일하다. 순수 날짜 계산이라 키·네트워크가 없다.
//!
//! 단정하지 않는 것: 조기전역·분할복무·휴가·공휴일은 반영하지 않는 *만기 전역일* 기준.

use chrono::{Months, NaiveDate};

/// 군별(복무기간과 호칭을 결정).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Army,     // 육군
    Marines,  // 해병대
    Navy,     // 해군
    AirForce, // 공군
    Social,   // 사회복무요원
}

impl Branch {
    /// 현행 복무기간(개월).
    pub fn months(self) -> u32 {
        match self {
            Branch::Army | Branch::Marines => 18,
            Branch::Navy => 20,
            Branch::AirForce | Branch::Social => 21,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Branch::Army => "육군",
            Branch::Marines => "해병대",
            Branch::Navy => "해군",
            Branch::AirForce => "공군",
            Branch::Social => "사회복무요원",
        }
    }

    /// 사회복무요원은 '소집해제', 현역은 '전역'.
    pub fn discharge_term(self) -> &'static str {
        match self {
            Branch::Social => "소집해제",
            _ => "전역",
        }
    }

    /// 병 계급 체계가 있는가(사회복무요원은 없음).
    pub fn has_rank(self) -> bool {
        self != Branch::Social
    }
}

/// `"육군"`·`"공군"`·`"공익"` 같은 입력을 `Branch`로 파싱한다.
pub fn parse_branch(s: &str) -> Result<Branch, String> {
    match s.trim() {
        "육군" | "육" | "army" => Ok(Branch::Army),
        "해병대" | "해병" | "marines" => Ok(Branch::Marines),
        "해군" | "해" | "navy" => Ok(Branch::Navy),
        "공군" | "공" | "airforce" | "air" => Ok(Branch::AirForce),
        "사회복무요원" | "사회복무" | "공익" | "social" => Ok(Branch::Social),
        other => Err(format!(
            "군별은 육군·해병대·해군·공군·사회복무요원 중 하나예요 (예: 전역 2025-03-04 육군). 입력: '{other}'"
        )),
    }
}

/// 병 계급(입대 후 경과 개월로 판정).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    Private,  // 이병
    Pfc,      // 일병
    Corporal, // 상병
    Sergeant, // 병장
}

impl Rank {
    pub fn label(self) -> &'static str {
        match self {
            Rank::Private => "이병",
            Rank::Pfc => "일병",
            Rank::Corporal => "상병",
            Rank::Sergeant => "병장",
        }
    }
}

fn add_months(d: NaiveDate, n: u32) -> NaiveDate {
    // chrono의 checked_add_months는 말일을 클램프한다(1/31 + 1개월 = 2/28).
    d.checked_add_months(Months::new(n)).unwrap()
}

/// 만기 전역일 = 입대일 + 복무월수 − 1일.
pub fn discharge_date(enlist: NaiveDate, branch: Branch) -> NaiveDate {
    add_months(enlist, branch.months()).pred_opt().unwrap()
}

/// 계급 진급 경계(입대 후 누적 개월): 일병 2, 상병 8, 병장 14.
const PROMO: [(u32, Rank); 3] = [(2, Rank::Pfc), (8, Rank::Corporal), (14, Rank::Sergeant)];

/// 오늘(포함) 기준 현재 계급.
pub fn rank_on(enlist: NaiveDate, today: NaiveDate) -> Rank {
    let mut rank = Rank::Private;
    for (m, r) in PROMO {
        if today >= add_months(enlist, m) {
            rank = r;
        }
    }
    rank
}

/// 다음 진급(계급, 진급일) — 이미 병장이거나 입대 전이면 가장 가까운 미래 경계.
/// 모두 지났으면(병장) None.
pub fn next_promotion(enlist: NaiveDate, today: NaiveDate) -> Option<(Rank, NaiveDate)> {
    PROMO
        .iter()
        .map(|&(m, r)| (r, add_months(enlist, m)))
        .find(|&(_, date)| today < date)
}

/// 복무 진행률(0~100). 입대 전이면 0, 전역 후면 100.
pub fn progress_pct(enlist: NaiveDate, branch: Branch, today: NaiveDate) -> u32 {
    let end = add_months(enlist, branch.months()); // 만기 경계(전역 다음날)
    let total = (end - enlist).num_days();
    if total <= 0 {
        return 100;
    }
    let elapsed = (today - enlist).num_days().clamp(0, total);
    (elapsed as f64 / total as f64 * 100.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn discharge_matches_known_examples() {
        // 알려진 검산: 2022-01-03 입대 육군 → 전역 2023-07-02.
        assert_eq!(discharge_date(d(2022, 1, 3), Branch::Army), d(2023, 7, 2));
        // 군별 복무기간.
        assert_eq!(Branch::Army.months(), 18);
        assert_eq!(Branch::Marines.months(), 18);
        assert_eq!(Branch::Navy.months(), 20);
        assert_eq!(Branch::AirForce.months(), 21);
        assert_eq!(Branch::Social.months(), 21);
        // 공군 21개월: 2023-05-10 입대 → 2025-02-09.
        assert_eq!(
            discharge_date(d(2023, 5, 10), Branch::AirForce),
            d(2025, 2, 9)
        );
    }

    #[test]
    fn discharge_clamps_month_end() {
        // 1/31 입대 육군: +18개월 = 2025-07-31, −1일 = 2025-07-30.
        assert_eq!(discharge_date(d(2024, 1, 31), Branch::Army), d(2025, 7, 30));
    }

    #[test]
    fn rank_progresses_by_elapsed_months() {
        let en = d(2025, 3, 4);
        assert_eq!(rank_on(en, d(2025, 3, 4)), Rank::Private); // 입대일
        assert_eq!(rank_on(en, d(2025, 5, 3)), Rank::Private); // +2개월 직전
        assert_eq!(rank_on(en, d(2025, 5, 4)), Rank::Pfc); // +2개월
        assert_eq!(rank_on(en, d(2025, 11, 4)), Rank::Corporal); // +8개월
        assert_eq!(rank_on(en, d(2026, 5, 4)), Rank::Sergeant); // +14개월
    }

    #[test]
    fn next_promotion_points_to_future_boundary() {
        let en = d(2025, 3, 4);
        assert_eq!(
            next_promotion(en, d(2025, 4, 1)),
            Some((Rank::Pfc, d(2025, 5, 4)))
        );
        assert_eq!(
            next_promotion(en, d(2025, 6, 1)),
            Some((Rank::Corporal, d(2025, 11, 4)))
        );
        // 병장(모든 경계 지남) → None.
        assert_eq!(next_promotion(en, d(2026, 6, 1)), None);
    }

    #[test]
    fn progress_clamps_to_0_100() {
        let en = d(2025, 3, 4);
        assert_eq!(progress_pct(en, Branch::Army, d(2024, 1, 1)), 0); // 입대 전
        assert_eq!(progress_pct(en, Branch::Army, d(2030, 1, 1)), 100); // 전역 후
        let mid = progress_pct(en, Branch::Army, d(2026, 1, 4)); // ~10개월/18
        assert!((50..=60).contains(&mid), "mid={mid}");
    }

    #[test]
    fn parse_branch_aliases() {
        assert_eq!(parse_branch("육군"), Ok(Branch::Army));
        assert_eq!(parse_branch("공익"), Ok(Branch::Social));
        assert_eq!(parse_branch("해병"), Ok(Branch::Marines));
        assert!(parse_branch("우주군").is_err());
    }
}
