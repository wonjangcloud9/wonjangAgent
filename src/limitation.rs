//! 소멸시효 — 채권 종류별 시효 기간과 완성(만료)일.
//!
//! 시효 기간은 민법·상법·근로기준법으로 오래 안정적이라 검증 가능하다.
//! 만료일 = 발생일(권리 행사 가능일) + 기간(년). 2/29는 평년 2/28로 클램프.
//!
//! 주의: **채권 종류 분류가 핵심**이며 사안마다 다툼이 있다(예: 같은 대여금도 상거래면
//! 상사 5년). 여기선 선택한 종류 기준의 일반적 기간만 — 구체 사안은 변호사 상담.

use chrono::{Months, NaiveDate};

/// 채권 종류별 시효(기간년, 근거 조문, 표시 라벨).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind {
    pub years: i64,
    pub basis: &'static str,
    pub label: &'static str,
}

/// `"상사"`·`"임금"`·`"음식"` 같은 입력을 채권 종류로(기본 민사 일반 10년).
pub fn parse_kind(s: &str) -> Result<Kind, String> {
    let k = match s.trim() {
        "민사" | "일반" | "대여금" | "빌린돈" | "빌려준돈" | "개인" => Kind {
            years: 10,
            basis: "민법 162조",
            label: "일반 민사채권",
        },
        "상사" | "상거래" | "카드" | "카드대금" | "사업" => Kind {
            years: 5,
            basis: "상법 64조",
            label: "상사채권(상거래)",
        },
        "임금" | "월급" | "퇴직금" | "급여" => Kind {
            years: 3,
            basis: "근로기준법 49조",
            label: "임금·퇴직금",
        },
        "물품" | "물품대금" | "공사" | "공사대금" | "용역" => Kind {
            years: 3,
            basis: "민법 163조",
            label: "물품·공사·용역대금",
        },
        "이자" | "월세" | "임대료" | "정기금" => Kind {
            years: 3,
            basis: "민법 163조",
            label: "이자·월세 등 정기금",
        },
        "보험" | "보험금" => Kind {
            years: 3,
            basis: "상법 662조",
            label: "보험금 청구권",
        },
        "음식" | "음식값" | "숙박" | "숙박료" | "여관" => Kind {
            years: 1,
            basis: "민법 164조",
            label: "음식·숙박료 등",
        },
        "판결" | "판결확정" | "확정판결" => Kind {
            years: 10,
            basis: "민법 165조",
            label: "판결로 확정된 채권",
        },
        other => {
            return Err(format!(
                "채권 종류: 민사·상사·임금·물품대금·이자·보험금·음식·판결 중 하나 (예: 소멸시효 2020-01-01 상사). 입력: '{other}'"
            ))
        }
    };
    Ok(k)
}

/// 발생일 + 기간(년) → 시효 완성(만료)일. 2/29 평년 클램프.
pub fn expiry(start: NaiveDate, years: i64) -> NaiveDate {
    start
        .checked_add_months(Months::new((years * 12) as u32))
        .unwrap_or(start)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn kinds_and_basis() {
        assert_eq!(parse_kind("민사").unwrap().years, 10);
        assert_eq!(parse_kind("상사").unwrap().years, 5);
        assert_eq!(parse_kind("임금").unwrap().years, 3);
        assert_eq!(parse_kind("음식").unwrap().years, 1);
        assert_eq!(parse_kind("판결").unwrap().basis, "민법 165조");
        assert!(parse_kind("우주").is_err());
    }

    #[test]
    fn expiry_dates() {
        assert_eq!(expiry(d(2020, 1, 1), 10), d(2030, 1, 1));
        assert_eq!(expiry(d(2020, 1, 1), 5), d(2025, 1, 1));
        assert_eq!(expiry(d(2020, 1, 1), 1), d(2021, 1, 1));
        // 2/29 → 평년 2/28 클램프.
        assert_eq!(expiry(d(2020, 2, 29), 10), d(2030, 2, 28));
    }
}
