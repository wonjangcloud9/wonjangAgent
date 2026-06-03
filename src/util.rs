//! 공용 유틸리티.

use anyhow::{Context, Result};
use std::future::Future;

/// 비동기 작업을 동기 컨텍스트에서 실행한다.
///
/// 도구 트레이트(`Tool::execute`)는 동기이지만 일부 도구(웹, 서브에이전트)는
/// 비동기 작업이 필요하다. 현재 스레드가 이미 tokio 런타임 안에 있을 수 있어,
/// 전용 스레드에서 새 런타임으로 실행해 "중첩 런타임" 패닉을 피한다.
pub fn run_async<T, F>(fut: F) -> Result<T>
where
    F: Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || -> Result<T> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("런타임 생성 실패")?;
        rt.block_on(fut)
    })
    .join()
    .map_err(|_| anyhow::anyhow!("작업 스레드가 패닉했습니다"))?
}

/// 문자열을 최대 `max_bytes` 바이트 이하에서 **UTF-8 문자 경계로** 안전하게 자른다.
///
/// `&s[..max_bytes]`는 한글·이모지 같은 멀티바이트 글자 중간을 자르면 패닉한다.
/// 이 헬퍼는 경계까지 되감아 안전한 앞부분과 잘렸는지 여부를 돌려준다.
/// 반환: (안전한 앞부분, 잘렸으면 true).
pub fn truncate_bytes(s: &str, max_bytes: usize) -> (&str, bool) {
    if s.len() <= max_bytes {
        return (s, false);
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    (&s[..end], true)
}

#[cfg(test)]
mod tests {
    use super::truncate_bytes;

    #[test]
    fn truncate_never_panics_on_multibyte_boundary() {
        // '가'는 3바이트. 모든 경계에서 패닉 없이 잘려야 한다.
        let s = "가".repeat(3000); // 9000바이트
        for n in [1, 2, 3, 4, 5, 100, 8000, 8001, 8002] {
            let (cut, truncated) = truncate_bytes(&s, n);
            assert!(cut.len() <= n);
            assert!(s.starts_with(cut)); // 유효한 앞부분
            assert!(truncated || cut.len() == s.len());
            // cut이 유효한 UTF-8(슬라이스가 패닉 안 했으면 보장됨)
            assert!(cut.chars().all(|c| c == '가'));
        }
    }

    #[test]
    fn truncate_short_string_unchanged() {
        let (cut, t) = truncate_bytes("안녕", 100);
        assert_eq!(cut, "안녕");
        assert!(!t);
    }

    #[test]
    fn truncate_ascii_exact() {
        let (cut, t) = truncate_bytes("hello world", 5);
        assert_eq!(cut, "hello");
        assert!(t);
    }
}
