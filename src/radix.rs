//! 진법 변환 — 2·8·10·16진수 상호 변환.
//!
//! 입력은 접두사로 진법을 자동 인식한다(0x=16, 0o=8, 0b=2, 그 외 10진수).
//! 개발·학습에 쓰며, 순수 변환이라 키가 없다.

use anyhow::{anyhow, Result};

/// 문자열을 접두사에 따라 해석해 정수로 바꾼다.
pub fn parse(input: &str) -> Result<u64> {
    let s = input.trim();
    let lower = s.to_lowercase();
    let bad = |_| anyhow!("'{s}'를 숫자로 해석할 수 없어요");
    if let Some(hex) = lower.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(bad)
    } else if let Some(oct) = lower.strip_prefix("0o") {
        u64::from_str_radix(oct, 8).map_err(bad)
    } else if let Some(bin) = lower.strip_prefix("0b") {
        u64::from_str_radix(bin, 2).map_err(bad)
    } else {
        s.parse::<u64>().map_err(bad)
    }
}

/// 네 가지 진법 표현(10·2·8·16).
pub struct Radixes {
    pub decimal: String,
    pub binary: String,
    pub octal: String,
    pub hex: String,
}

/// 정수를 네 가지 진법 표현으로.
pub fn all(n: u64) -> Radixes {
    Radixes {
        decimal: n.to_string(),
        binary: format!("0b{n:b}"),
        octal: format!("0o{n:o}"),
        hex: format!("0x{n:X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_by_prefix() {
        assert_eq!(parse("255").unwrap(), 255);
        assert_eq!(parse("0xFF").unwrap(), 255);
        assert_eq!(parse("0o377").unwrap(), 255);
        assert_eq!(parse("0b11111111").unwrap(), 255);
    }

    #[test]
    fn formats_all_bases() {
        let r = all(255);
        assert_eq!(r.binary, "0b11111111");
        assert_eq!(r.octal, "0o377");
        assert_eq!(r.hex, "0xFF");
    }

    #[test]
    fn round_trip() {
        let r = all(3735928559); // 0xDEADBEEF
        assert_eq!(r.hex, "0xDEADBEEF");
        assert_eq!(parse(&r.hex).unwrap(), 3735928559);
    }

    #[test]
    fn rejects_bad() {
        assert!(parse("0xZZ").is_err());
        assert!(parse("abc").is_err());
    }
}
