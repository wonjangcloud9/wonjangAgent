//! UUID v4 생성 — DB 기본키·요청 ID 등에 쓰는 무작위 UUID.
//!
//! RFC 4122 버전 4(완전 무작위)를 OS 난수(getrandom)로 만든다. 진짜 무작위라
//! GPT로는 안전하게 만들 수 없다. 새 의존성 없이 직접 포맷한다.

use anyhow::{anyhow, Result};

/// UUID v4 하나를 만든다(예: 550e8400-e29b-41d4-a716-446655440000).
pub fn v4() -> Result<String> {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).map_err(|e| anyhow!("OS 난수 실패: {e}"))?;
    // 버전(4)과 변형(RFC 4122) 비트 설정.
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_is_valid() {
        let u = v4().unwrap();
        // 36자, 하이픈 위치, 16진수.
        assert_eq!(u.len(), 36);
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(
            parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
            vec![8, 4, 4, 4, 12]
        );
        assert!(u.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn version_and_variant_bits() {
        let u = v4().unwrap();
        // 세 번째 그룹 첫 글자는 '4'(버전 4).
        let third = u.split('-').nth(2).unwrap();
        assert!(third.starts_with('4'));
        // 네 번째 그룹 첫 글자는 8/9/a/b(변형).
        let fourth = u.split('-').nth(3).unwrap();
        assert!(matches!(
            fourth.chars().next().unwrap(),
            '8' | '9' | 'a' | 'b'
        ));
    }

    #[test]
    fn unique() {
        assert_ne!(v4().unwrap(), v4().unwrap());
    }
}
