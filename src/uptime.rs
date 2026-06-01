//! 사이트 상태 확인 — "내 사이트/서버 살아있나? 얼마나 빠르지?"를 점검한다.
//!
//! URL에 HTTP 요청을 보내 상태 코드와 응답 시간을 잰다. 내 네트워크에서 실제로
//! 접속해 보는 것이라 GPT로는 알 수 없다. 키가 필요 없다.

use anyhow::{anyhow, Context, Result};
use std::time::Instant;

/// 점검 결과.
pub struct Status {
    pub url: String,      // 최종 URL(리다이렉트 반영)
    pub code: u16,        // HTTP 상태 코드
    pub ok: bool,         // 2xx 여부
    pub elapsed_ms: u128, // 응답 시간(ms)
}

/// http/https 접두사를 보정한다.
fn normalize(url: &str) -> String {
    let u = url.trim();
    if u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else {
        format!("https://{u}")
    }
}

/// URL의 상태와 응답 시간을 잰다.
pub async fn check(url: &str) -> Result<Status> {
    let url = normalize(url);
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("wonjang-agent")
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let start = Instant::now();
    let resp = http
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("접속 실패(서버가 꺼졌거나 주소가 틀렸을 수 있어요): {e}"))?;
    let elapsed_ms = start.elapsed().as_millis();
    let code = resp.status().as_u16();
    Ok(Status {
        url: resp.url().to_string(),
        ok: resp.status().is_success(),
        code,
        elapsed_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_scheme() {
        assert_eq!(normalize("example.com"), "https://example.com");
        assert_eq!(normalize("http://a.com"), "http://a.com");
        assert_eq!(normalize(" https://b.com "), "https://b.com");
    }
}
