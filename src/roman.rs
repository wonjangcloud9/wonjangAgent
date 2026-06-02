//! 로마 숫자 변환 — 연도·챕터·시계 표기 등에 쓰는 숫자 ↔ 로마자(1~3999).
//!
//! 표준 가감 표기(예: 4=IV, 9=IX, 2024=MMXXIV)를 따른다. 순수 계산이라 키가 없다.

use anyhow::{anyhow, Result};

const TABLE: &[(u32, &str)] = &[
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];

/// 정수(1~3999)를 로마 숫자로.
pub fn to_roman(mut n: u32) -> Result<String> {
    if n == 0 || n > 3999 {
        return Err(anyhow!("로마 숫자는 1~3999만 표현돼요"));
    }
    let mut out = String::new();
    for (val, sym) in TABLE {
        while n >= *val {
            out.push_str(sym);
            n -= val;
        }
    }
    Ok(out)
}

/// 로마 숫자를 정수로.
pub fn from_roman(s: &str) -> Result<u32> {
    let s = s.trim().to_uppercase();
    if s.is_empty() || !s.chars().all(|c| "IVXLCDM".contains(c)) {
        return Err(anyhow!("로마 숫자(I V X L C D M)만 입력하세요"));
    }
    let val = |c: char| match c {
        'I' => 1,
        'V' => 5,
        'X' => 10,
        'L' => 50,
        'C' => 100,
        'D' => 500,
        'M' => 1000,
        _ => 0,
    };
    let chars: Vec<char> = s.chars().collect();
    let mut total = 0i64;
    for i in 0..chars.len() {
        let cur = val(chars[i]);
        let next = chars.get(i + 1).map(|c| val(*c)).unwrap_or(0);
        if cur < next {
            total -= cur;
        } else {
            total += cur;
        }
    }
    if !(1..=3999).contains(&total) {
        return Err(anyhow!("1~3999 범위를 벗어났어요"));
    }
    // 왕복 검증(잘못된 표기 거르기, 예: IIII).
    if to_roman(total as u32)? != s {
        return Err(anyhow!("올바른 로마 숫자 표기가 아니에요"));
    }
    Ok(total as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_roman_known() {
        assert_eq!(to_roman(4).unwrap(), "IV");
        assert_eq!(to_roman(9).unwrap(), "IX");
        assert_eq!(to_roman(2024).unwrap(), "MMXXIV");
        assert_eq!(to_roman(3999).unwrap(), "MMMCMXCIX");
    }

    #[test]
    fn from_roman_known() {
        assert_eq!(from_roman("IV").unwrap(), 4);
        assert_eq!(from_roman("mmxxiv").unwrap(), 2024);
    }

    #[test]
    fn rejects_invalid() {
        assert!(to_roman(0).is_err());
        assert!(to_roman(4000).is_err());
        assert!(from_roman("IIII").is_err()); // 4는 IV
        assert!(from_roman("ABC").is_err());
    }
}
