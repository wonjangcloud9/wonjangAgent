//! 웹 도구의 코어 로직 — 웹 검색과 페이지 가져오기.
//!
//! 도구 트레이트(`Tool::execute`)는 동기이므로, 비동기 HTTP 요청은 전용
//! 스레드에서 새 런타임으로 실행해 중첩 런타임 패닉을 피한다.
//!
//! 검색은 별도 API 키 없이 DuckDuckGo HTML 엔드포인트를 사용한다(베스트 에포트).

use anyhow::{Context, Result};
use std::future::Future;

const UA: &str = "Mozilla/5.0 (compatible; wonjang-agent/0.1; +https://github.com/wonjangcloud9/wonjangAgent)";

/// 비동기 작업을 동기 컨텍스트에서 실행한다(전용 스레드 + current-thread 런타임).
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
    .map_err(|_| anyhow::anyhow!("웹 작업 스레드가 패닉했습니다"))?
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .context("HTTP 클라이언트 생성 실패")
}

/// 검색 결과 한 건.
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// DuckDuckGo HTML로 웹 검색(상위 `limit`건).
pub async fn search(query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let client = client()?;
    let resp = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .await
        .context("검색 요청 실패")?;
    let body = resp.text().await.context("검색 응답 읽기 실패")?;
    Ok(parse_ddg(&body, limit))
}

/// URL의 본문을 가져와 텍스트만 추출.
pub async fn fetch(url: &str, max_len: usize) -> Result<String> {
    let client = client()?;
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("페이지 요청 실패: {url}"))?;
    let status = resp.status();
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = resp.text().await.context("페이지 응답 읽기 실패")?;

    let text = if ctype.contains("html") || body.trim_start().starts_with('<') {
        html_to_text(&body)
    } else {
        body
    };
    let mut out = format!("[{status}] {url}\n\n");
    if text.len() > max_len {
        out.push_str(&text[..max_len]);
        out.push_str(&format!("\n… (본문이 길어 {max_len}자에서 잘림)"));
    } else {
        out.push_str(&text);
    }
    Ok(out)
}

// ── DuckDuckGo HTML 파싱 ─────────────────────────────────────────────────

fn parse_ddg(html: &str, limit: usize) -> Vec<SearchResult> {
    let titles_urls = extract_anchors(html, "result__a");
    let snippets = extract_anchor_texts(html, "result__snippet");

    let mut results = Vec::new();
    for (i, (title, href)) in titles_urls.into_iter().enumerate() {
        if i >= limit {
            break;
        }
        let url = decode_ddg_url(&href);
        let snippet = snippets.get(i).cloned().unwrap_or_default();
        if !title.is_empty() {
            results.push(SearchResult { title, url, snippet });
        }
    }
    results
}

/// 지정 class를 가진 앵커들의 (텍스트, href)를 추출.
fn extract_anchors(html: &str, class: &str) -> Vec<(String, String)> {
    let needle = format!("class=\"{class}\"");
    let mut out = Vec::new();
    for seg in html.split(&needle).skip(1) {
        // href는 class 앞/뒤 어디든 올 수 있어, 같은 <a ...> 태그 범위에서 찾는다.
        // 안전하게: 이 세그먼트 앞쪽(태그 시작)에서 href를 역방향 탐색하기 어렵기에,
        // DuckDuckGo는 href가 class 뒤에 오는 경우가 일반적이므로 뒤에서 찾는다.
        let href = find_attr_after(seg, "href=\"").unwrap_or_default();
        let text = inner_text_until(seg, "</a>");
        out.push((strip_tags(&text), href));
    }
    out
}

fn extract_anchor_texts(html: &str, class: &str) -> Vec<String> {
    let needle = format!("class=\"{class}\"");
    let mut out = Vec::new();
    for seg in html.split(&needle).skip(1) {
        let text = inner_text_until(seg, "</a>");
        out.push(strip_tags(&text));
    }
    out
}

/// 세그먼트에서 `attr` 다음의 값(다음 `"`까지)을 찾는다.
fn find_attr_after(seg: &str, attr: &str) -> Option<String> {
    let start = seg.find(attr)? + attr.len();
    let rest = &seg[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// `>` 이후부터 `end` 전까지의 내부 텍스트.
fn inner_text_until(seg: &str, end: &str) -> String {
    let start = match seg.find('>') {
        Some(i) => i + 1,
        None => 0,
    };
    let rest = &seg[start..];
    let stop = rest.find(end).unwrap_or(rest.len());
    rest[..stop].to_string()
}

/// DuckDuckGo 리다이렉트 URL(`...uddg=<encoded>`)을 실제 URL로 디코딩.
fn decode_ddg_url(href: &str) -> String {
    if let Some(idx) = href.find("uddg=") {
        let rest = &href[idx + 5..];
        let enc = rest.split('&').next().unwrap_or(rest);
        return percent_decode(enc);
    }
    if let Some(stripped) = href.strip_prefix("//") {
        format!("https://{stripped}")
    } else {
        href.to_string()
    }
}

/// 퍼센트 디코딩(+ → 공백 포함).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let h = hex_val(bytes[i + 1]);
                let l = hex_val(bytes[i + 2]);
                if let (Some(h), Some(l)) = (h, l) {
                    out.push(h * 16 + l);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ── HTML → 텍스트 ─────────────────────────────────────────────────────────

/// HTML을 대략적인 일반 텍스트로 변환(script/style 제거, 태그 제거, 공백 정리).
pub fn html_to_text(html: &str) -> String {
    let without_blocks = remove_blocks(html, &["script", "style", "noscript", "head"]);
    let stripped = strip_tags(&without_blocks);
    let decoded = decode_entities(&stripped);
    collapse_ws(&decoded)
}

/// `<tag>...</tag>` 블록 전체를 제거.
fn remove_blocks(html: &str, tags: &[&str]) -> String {
    let mut s = html.to_string();
    for tag in tags {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        loop {
            let lower = s.to_lowercase();
            if let Some(start) = lower.find(&open) {
                if let Some(rel_end) = lower[start..].find(&close) {
                    let end = start + rel_end + close.len();
                    s.replace_range(start..end, " ");
                    continue;
                }
            }
            break;
        }
    }
    s
}

/// 모든 `<...>` 태그를 공백으로 치환.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// 자주 쓰이는 HTML 엔티티 디코딩.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

/// 연속 공백/빈 줄 정리.
fn collapse_ws(s: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in s.lines() {
        let t: String = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !t.is_empty() {
            lines.push(t);
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_basic() {
        assert_eq!(percent_decode("https%3A%2F%2Fa.com%2Fb"), "https://a.com/b");
        assert_eq!(percent_decode("a+b"), "a b");
    }

    #[test]
    fn decode_ddg_extracts_real_url() {
        let href = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpath&rut=abc";
        assert_eq!(decode_ddg_url(href), "https://example.com/path");
    }

    #[test]
    fn html_to_text_strips_tags_and_scripts() {
        let html = "<html><head><title>t</title></head><body><script>var x=1;</script><p>안녕 &amp; 반가워</p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("안녕 & 반가워"));
        assert!(!text.contains("var x"));
    }

    // 네트워크가 필요한 라이브 테스트(기본 비활성). 실행: `cargo test -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn live_fetch_example() {
        let text = fetch("https://example.com", 5000).await.unwrap();
        assert!(text.contains("Example Domain"), "본문: {text}");
    }

    #[tokio::test]
    #[ignore]
    async fn live_search_returns_results() {
        let results = search("rust 프로그래밍 언어", 5).await.unwrap();
        assert!(!results.is_empty(), "검색 결과가 비어 있음");
        assert!(results[0].url.starts_with("http"), "URL: {}", results[0].url);
    }
}
