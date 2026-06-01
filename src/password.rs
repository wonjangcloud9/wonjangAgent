//! 안전한 비밀번호 생성 — OS 난수로 암호학적으로 안전하게 만든다.
//!
//! `getrandom`(운영체제 난수)을 써서 진짜 무작위를 보장한다(시간 시드 PRNG가
//! 아님). 선택한 문자 종류가 최소 1개씩 포함되도록 하고, 모듈로 편향을 없애기
//! 위해 거부 표본추출(rejection sampling)을 쓴다.

use anyhow::{anyhow, Result};

const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*-_=+?";

/// 0..max 범위의 균등 난수(거부 표본추출로 편향 제거).
fn uniform(max: u8) -> Result<u8> {
    debug_assert!(max > 0);
    let limit = (256u16 - (256u16 % max as u16)) as u8; // max의 배수 경계
    loop {
        let mut b = [0u8; 1];
        getrandom::getrandom(&mut b).map_err(|e| anyhow!("OS 난수 실패: {e}"))?;
        // limit==0이면 max==... 처리: max<=128일 때만 안전하게 동작.
        if limit == 0 || b[0] < limit {
            return Ok(b[0] % max);
        }
    }
}

/// 길이와 문자 종류를 받아 비밀번호를 만든다.
pub fn generate(length: usize, symbols: bool) -> Result<String> {
    let length = length.clamp(4, 128);
    // 사용할 문자 종류.
    let mut classes: Vec<&[u8]> = vec![LOWER, UPPER, DIGITS];
    if symbols {
        classes.push(SYMBOLS);
    }
    let pool: Vec<u8> = classes.iter().flat_map(|c| c.iter().copied()).collect();

    let mut out: Vec<u8> = Vec::with_capacity(length);
    // 각 종류에서 최소 1개씩 먼저 채운다.
    for class in &classes {
        let idx = uniform(class.len() as u8)? as usize;
        out.push(class[idx]);
    }
    // 나머지는 전체 풀에서.
    while out.len() < length {
        let idx = uniform(pool.len() as u8)? as usize;
        out.push(pool[idx]);
    }
    // 앞쪽에 몰린 종류 보장 문자를 섞는다(Fisher-Yates, OS 난수).
    for i in (1..out.len()).rev() {
        let j = uniform((i + 1) as u8)? as usize;
        out.swap(i, j);
    }
    Ok(String::from_utf8(out).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_is_respected() {
        assert_eq!(generate(16, true).unwrap().chars().count(), 16);
        assert_eq!(generate(8, false).unwrap().chars().count(), 8);
        // 최소 길이 클램프.
        assert_eq!(generate(1, false).unwrap().chars().count(), 4);
    }

    #[test]
    fn contains_each_class() {
        let p = generate(20, true).unwrap();
        assert!(p.bytes().any(|b| b.is_ascii_lowercase()));
        assert!(p.bytes().any(|b| b.is_ascii_uppercase()));
        assert!(p.bytes().any(|b| b.is_ascii_digit()));
        assert!(p.bytes().any(|b| SYMBOLS.contains(&b)));
    }

    #[test]
    fn no_symbols_when_disabled() {
        let p = generate(30, false).unwrap();
        assert!(p.bytes().all(|b| b.is_ascii_alphanumeric()));
    }

    #[test]
    fn two_generations_differ() {
        // 무작위라 같을 확률은 사실상 0.
        assert_ne!(generate(24, true).unwrap(), generate(24, true).unwrap());
    }
}
