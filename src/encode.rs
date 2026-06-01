//! 인코딩/디코딩 — base64·URL 퍼센트 인코딩(개발자 필수 유틸).
//!
//! JWT 페이로드 디코딩, URL 파라미터 인코딩 등에 쓴다. base64는 검증된
//! 크레이트, URL 퍼센트 인코딩은 RFC 3986 비예약 문자 기준으로 직접 구현.

use anyhow::{anyhow, Result};
use base64::Engine;

/// base64 인코딩.
pub fn base64_encode(input: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

/// base64 디코딩(→ UTF-8 문자열).
pub fn base64_decode(input: &str) -> Result<String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.trim())
        .map_err(|e| anyhow!("base64가 아니에요: {e}"))?;
    String::from_utf8(bytes).map_err(|_| anyhow!("디코딩 결과가 텍스트가 아니에요(바이너리)"))
}

/// URL 퍼센트 인코딩(비예약 문자 A-Za-z0-9-_.~ 만 그대로).
pub fn url_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// URL 퍼센트 디코딩.
pub fn url_decode(input: &str) -> Result<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(anyhow!("잘못된 % 인코딩"));
                }
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                let v =
                    u8::from_str_radix(hex, 16).map_err(|_| anyhow!("잘못된 % 인코딩: %{hex}"))?;
                out.push(v);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| anyhow!("디코딩 결과가 텍스트가 아니에요"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        assert_eq!(base64_encode("hello"), "aGVsbG8=");
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), "hello");
        assert_eq!(
            base64_decode(&base64_encode("한글 테스트")).unwrap(),
            "한글 테스트"
        );
    }

    #[test]
    fn url_roundtrip() {
        assert_eq!(url_encode("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(url_decode("a%20b%26c%3Dd").unwrap(), "a b&c=d");
        assert_eq!(url_decode("a+b").unwrap(), "a b");
        // 한글.
        assert_eq!(url_decode(&url_encode("안녕")).unwrap(), "안녕");
    }

    #[test]
    fn errors() {
        assert!(base64_decode("!!!not base64!!!").is_err());
        assert!(url_decode("%ZZ").is_err());
    }
}
