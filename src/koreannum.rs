//! 숫자 → 한글 금액 표기 — 계약서·세금계산서·수표용.
//!
//! 위·변조 방지를 위해 금액을 한글로 적을 때(예: 1,234,567 →
//! "일백이십삼만사천오백육십칠") 쓰는 형식. 자리 숫자 1도 일십·일백·일천처럼
//! 명시하는 정식(격식) 표기를 따른다. 순수 변환이라 키가 없다.

const DIGITS: [&str; 10] = ["영", "일", "이", "삼", "사", "오", "육", "칠", "팔", "구"];
const SMALL_UNITS: [&str; 4] = ["", "십", "백", "천"];
const BIG_UNITS: [&str; 5] = ["", "만", "억", "조", "경"];

/// 4자리 이하 묶음을 한글로(예: 4567 → 사천오백육십칠).
fn group_to_korean(g: u64) -> String {
    let mut s = String::new();
    for pos in (0..4).rev() {
        let digit = (g / 10u64.pow(pos as u32)) % 10;
        if digit != 0 {
            s.push_str(DIGITS[digit as usize]);
            s.push_str(SMALL_UNITS[pos]);
        }
    }
    s
}

/// 0 이상의 정수를 한글 금액 표기로 바꾼다.
pub fn to_korean(n: u64) -> String {
    if n == 0 {
        return "영".to_string();
    }
    // 4자리씩 묶어 작은 자리부터 모은다.
    let mut groups = Vec::new();
    let mut rest = n;
    while rest > 0 {
        groups.push(rest % 10_000);
        rest /= 10_000;
    }
    // 큰 자리부터 이어 붙인다.
    let mut out = String::new();
    for idx in (0..groups.len()).rev() {
        let g = groups[idx];
        if g == 0 {
            continue;
        }
        out.push_str(&group_to_korean(g));
        out.push_str(BIG_UNITS[idx]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_numbers() {
        assert_eq!(to_korean(0), "영");
        assert_eq!(to_korean(7), "칠");
        assert_eq!(to_korean(10), "일십");
        assert_eq!(to_korean(123), "일백이십삼");
    }

    #[test]
    fn amount_notation() {
        assert_eq!(to_korean(1_234_567), "일백이십삼만사천오백육십칠");
        assert_eq!(to_korean(10_000), "일만");
        assert_eq!(to_korean(100_200), "일십만이백");
    }

    #[test]
    fn skips_empty_groups() {
        // 1억 = 일억 (중간 만 자리 0 생략).
        assert_eq!(to_korean(100_000_000), "일억");
        assert_eq!(to_korean(100_010_000), "일억일만");
    }

    #[test]
    fn large_units() {
        assert_eq!(to_korean(1_0000_0000_0000u64), "일조");
    }
}
