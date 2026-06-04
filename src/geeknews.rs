//! 긱뉴스(GeekNews) — 개발·기술·스타트업 뉴스를 공개 RSS로 가져온다(무료, 키 불필요).
//!
//! 개발자에게 유용한 한국 기술 뉴스 모음(news.hada.io). 일반 뉴스(`뉴스`)와 달리
//! 개발/스타트업 위주다. Atom 피드를 파싱한다.

use anyhow::{Context, Result};

/// 긱뉴스 항목.
pub struct Item {
    pub title: String,
    pub link: String,
}

/// 최신 긱뉴스를 가져온다.
pub async fn fetch(limit: usize) -> Result<Vec<Item>> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; wonjang-agent)")
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let body = http
        .get("https://news.hada.io/rss/news")
        .send()
        .await
        .context("긱뉴스 요청 실패")?
        .text()
        .await
        .context("긱뉴스 응답 읽기 실패")?;
    Ok(parse(&body, limit))
}

/// Atom 피드에서 entry별 제목·링크를 뽑는다.
fn parse(atom: &str, limit: usize) -> Vec<Item> {
    let mut out = Vec::new();
    for entry in atom.split("<entry>").skip(1) {
        if out.len() >= limit {
            break;
        }
        let title = extract_tag(entry, "title")
            .map(|t| clean(&t))
            .unwrap_or_default();
        let link = extract_href(entry).unwrap_or_default();
        if !title.is_empty() {
            out.push(Item { title, link });
        }
    }
    out
}

fn extract_tag(s: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = s.find(&open)? + open.len();
    let end = s[start..].find(&close)? + start;
    Some(s[start..end].to_string())
}

/// 첫 `href='...'`(또는 `href="..."`) 값을 뽑는다.
fn extract_href(s: &str) -> Option<String> {
    let pos = s.find("href=")? + "href=".len();
    let rest = &s[pos..];
    let quote = rest.chars().next()?; // ' 또는 "
                                      // 첫 글자가 멀티바이트(따옴표 없이 비ASCII)면 &rest[1..]가 바이트 경계 패닉 → len_utf8로 안전하게.
    let inner = &rest[quote.len_utf8()..];
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_href_no_panic_on_multibyte() {
        // 따옴표 없이 비ASCII로 시작하는 href(변형 피드) → 바이트 경계 패닉 없이 안전.
        assert_eq!(extract_href("href=가나다"), None); // 닫는 따옴표 없음 → None(패닉 X)
        assert_eq!(
            extract_href("href='https://a.com'"),
            Some("https://a.com".into())
        );
        assert_eq!(extract_href("href=\"x\""), Some("x".into()));
    }

    #[test]
    fn parses_atom_entries() {
        let atom = r#"
        <feed>
        <entry>
        <title>Rust 1.99 출시</title>
        <link rel='alternate' type='text/html' href='https://news.hada.io/topic?id=1'/>
        </entry>
        <entry>
        <title>스타트업 &amp; 투자 동향</title>
        <link rel='alternate' href='https://news.hada.io/topic?id=2'/>
        </entry>
        </feed>"#;
        let items = parse(atom, 10);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Rust 1.99 출시");
        assert_eq!(items[0].link, "https://news.hada.io/topic?id=1");
        assert_eq!(items[1].title, "스타트업 & 투자 동향");
    }

    #[test]
    fn respects_limit() {
        let atom = "<entry><title>a</title></entry><entry><title>b</title></entry>";
        assert_eq!(parse(atom, 1).len(), 1);
    }
}
