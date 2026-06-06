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

/// 파일을 **원자적으로** 쓴다(임시 파일에 쓴 뒤 rename).
///
/// `std::fs::write`는 쓰는 도중 끊기면(크래시·동시 쓰기) 파일이 반쯤 쓰여 깨진다.
/// 데몬(예약 작업)과 CLI가 같은 저장소를 동시에 쓸 수 있으므로, 임시 파일에 다 쓴 뒤
/// 같은 디렉터리 안에서 rename(원자적)으로 교체해 **반쯤 쓰인 깨진 파일이 남지 않게** 한다.
/// 임시 파일명에 프로세스 id를 붙여 동시 쓰기 시 임시 파일끼리도 충돌하지 않는다.
pub fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    // rename 실패 시 임시 파일을 정리(고아 tmp 누수 방지).
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// JSON 저장소를 읽어 역직렬화한다 — **손상 시 조용히 비우지 않는다**.
///
/// 기존 패턴 `from_str(&text).unwrap_or_default()`는 파일에 필수 필드가 빠진 항목이
/// 하나라도 있으면 배열 전체 파싱이 실패해 **빈 저장소**가 되고, 다음 save가 파일을
/// 통째로 덮어써 **모든 데이터가 조용히 사라진다**(외부 편집·디스크 손상 시).
///
/// 이 헬퍼는 파싱 실패 시 손상 파일을 `*.corrupt`로 **보존**(복구 가능)하고 기본값을
/// 돌려준다 — 명령은 계속 동작하되 원본 데이터는 잃지 않는다.
pub fn load_json_or_recover<T>(path: &std::path::Path) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    if !path.exists() {
        return T::default();
    }
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return T::default(),
    };
    if text.trim().is_empty() {
        return T::default();
    }
    match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            // 손상 파일을 .corrupt로 백업한 뒤 기본값으로(덮어쓰기 전 보존 → 무음 손실 방지).
            let backup = path.with_extension("corrupt");
            if std::fs::rename(path, &backup).is_ok() {
                eprintln!(
                    "  ⚠ 저장 파일이 손상돼 {}로 옮기고 새로 시작해요({e}). 복구하려면 그 파일을 확인하세요.",
                    backup.display()
                );
            }
            T::default()
        }
    }
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

/// 한국식 시간/기간 표기를 분(minutes) 정수로 파싱한다(clap value_parser용).
/// `30`=30분 · `30분`=30 · `1시간`=60 · `1시간 30분`=90 · `1.5시간`=90.
/// 순수 숫자는 그대로 분으로(기존 동작 유지). 못 알아들으면 한국어로 예시 안내.
pub fn parse_minutes(s: &str) -> std::result::Result<i64, String> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return Err("시간을 입력하세요 (예: 30, 30분, 1시간, 1시간 30분)".into());
    }
    if let Ok(n) = t.parse::<i64>() {
        return Ok(n); // 순수 숫자 = 분
    }
    let err = || format!("시간을 이해하지 못했어요: '{s}' (예: 30, 30분, 1시간, 1시간 30분)");
    let mut total = 0i64;
    let mut rest = t.as_str();
    let mut matched = false;
    if let Some((h, b)) = rest.split_once("시간") {
        let hv: f64 = h.parse().map_err(|_| err())?;
        total += (hv * 60.0).round() as i64;
        rest = b;
        matched = true;
    }
    if let Some((m, b)) = rest.split_once('분') {
        if !b.is_empty() {
            return Err(err()); // '분' 뒤에 찌꺼기
        }
        if !m.is_empty() {
            total += m.parse::<i64>().map_err(|_| err())?;
        }
        rest = "";
        matched = true;
    }
    if !matched || !rest.is_empty() {
        return Err(err());
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::{parse_minutes, truncate_bytes};

    #[test]
    fn parse_minutes_accepts_korean_time() {
        assert_eq!(parse_minutes("30"), Ok(30));
        assert_eq!(parse_minutes("30분"), Ok(30));
        assert_eq!(parse_minutes("1시간"), Ok(60));
        assert_eq!(parse_minutes("2시간"), Ok(120));
        assert_eq!(parse_minutes("1시간 30분"), Ok(90));
        assert_eq!(parse_minutes("1시간30분"), Ok(90));
        assert_eq!(parse_minutes("1.5시간"), Ok(90));
        assert_eq!(parse_minutes(" 45 분 "), Ok(45));
        // 못 알아듣는 입력은 한국어 안내(영문 clap 에러 대신).
        assert!(parse_minutes("한시간").is_err());
        assert!(parse_minutes("abc").is_err());
        assert!(parse_minutes("1시간 abc").is_err());
        assert!(parse_minutes("").is_err());
    }

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
