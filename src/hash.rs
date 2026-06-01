//! 파일 체크섬(SHA-256/512) — 다운로드 파일 무결성 검증 등에 쓴다.
//!
//! `shasum -a 256 <파일>`과 같은 값을 낸다. 큰 파일도 스트리밍으로 읽어 메모리를
//! 아낀다. 순수 Rust(sha2)라 모든 플랫폼에서 빌드된다. 네트워크·키가 필요 없다.

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256, Sha512};
use std::io::Read;
use std::path::Path;

/// 지원 알고리즘.
#[derive(Clone, Copy)]
pub enum Algo {
    Sha256,
    Sha512,
}

impl Algo {
    pub fn parse(s: &str) -> Result<Algo> {
        match s.trim().to_lowercase().as_str() {
            "sha256" | "256" => Ok(Algo::Sha256),
            "sha512" | "512" => Ok(Algo::Sha512),
            other => Err(anyhow!("지원하지 않는 알고리즘: {other} (sha256/sha512)")),
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Algo::Sha256 => "SHA-256",
            Algo::Sha512 => "SHA-512",
        }
    }
}

/// 파일의 체크섬을 16진 문자열로 계산한다.
pub fn file_digest(path: &Path, algo: Algo) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("파일을 열 수 없어요: {}", path.display()))?;
    let mut reader = std::io::BufReader::new(file);
    let mut buf = [0u8; 16 * 1024];
    match algo {
        Algo::Sha256 => {
            let mut h = Sha256::new();
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(hex(&h.finalize()))
        }
        Algo::Sha512 => {
            let mut h = Sha512::new();
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(hex(&h.finalize()))
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_known_string() {
        let base = std::env::temp_dir().join("wonjang_hash_test.txt");
        std::fs::write(&base, b"hello").unwrap();
        let d = file_digest(&base, Algo::Sha256).unwrap();
        // "hello"의 SHA-256(잘 알려진 값).
        assert_eq!(
            d,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        let _ = std::fs::remove_file(&base);
    }

    #[test]
    fn algo_parse() {
        assert!(matches!(Algo::parse("sha256").unwrap(), Algo::Sha256));
        assert!(matches!(Algo::parse("512").unwrap(), Algo::Sha512));
        assert!(Algo::parse("md5").is_err());
    }
}
