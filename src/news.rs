//! 뉴스 헤드라인 — 구글뉴스(한국) RSS로 최신 뉴스를 가져온다(무료, 키 불필요).
//!
//! 아침 브리핑에 "오늘의 뉴스"를 더하고, 키워드 검색도 지원한다.

use anyhow::{Context, Result};

/// 헤드라인을 가져온다(query 있으면 검색, 없으면 주요 뉴스).
pub async fn headlines(query: Option<&str>, limit: usize) -> Result<Vec<String>> {
    let url = match query {
        Some(q) if !q.trim().is_empty() => format!(
            "https://news.google.com/rss/search?q={}&hl=ko&gl=KR&ceid=KR:ko",
            encode(q.trim())
        ),
        _ => "https://news.google.com/rss?hl=ko&gl=KR&ceid=KR:ko".to_string(),
    };
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; wonjang-agent)")
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let body = http
        .get(&url)
        .send()
        .await
        .context("뉴스 요청 실패")?
        .text()
        .await
        .context("뉴스 응답 읽기 실패")?;
    Ok(parse_titles(&body, limit))
}

/// RSS 본문에서 item 제목들을 추출한다(채널 제목 제외).
fn parse_titles(rss: &str, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    // 첫 조각은 채널 헤더(<item> 이전) → skip(1).
    for item in rss.split("<item>").skip(1) {
        if out.len() >= limit {
            break;
        }
        if let Some(title) = extract_tag(item, "title") {
            let cleaned = clean(&title);
            if !cleaned.is_empty() {
                out.push(cleaned);
            }
        }
    }
    out
}

/// `<tag>...</tag>` 안쪽 텍스트.
fn extract_tag(s: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = s.find(&open)? + open.len();
    let end = s[start..].find(&close)? + start;
    Some(s[start..end].to_string())
}

/// CDATA·엔티티 정리.
fn clean(s: &str) -> String {
    s.replace("<![CDATA[", "")
        .replace("]]>", "")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .trim()
        .to_string()
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skips_channel_title() {
        let rss = "<channel><title>Google 뉴스</title>\
            <item><title>헤드라인 하나 - 한겨레</title></item>\
            <item><title><![CDATA[헤드라인 둘 &amp; 셋]]></title></item></channel>";
        let titles = parse_titles(rss, 10);
        assert_eq!(titles.len(), 2);
        assert_eq!(titles[0], "헤드라인 하나 - 한겨레");
        assert_eq!(titles[1], "헤드라인 둘 & 셋");
    }

    #[test]
    fn respects_limit() {
        let rss = "<item><title>1</title></item><item><title>2</title></item><item><title>3</title></item>";
        assert_eq!(parse_titles(rss, 2).len(), 2);
    }

    #[tokio::test]
    #[ignore]
    async fn live_headlines() {
        let h = headlines(None, 5).await.unwrap();
        assert!(!h.is_empty());
    }
}
