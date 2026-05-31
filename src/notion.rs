//! 노션(Notion) 연동 — 한국 사용자가 많이 쓰는 노션 워크스페이스를 다룬다.
//!
//! 노션 통합 토큰으로 페이지/DB를 검색하고, 페이지에 내용을 덧붙인다.
//! 토큰은 비밀값이라 환경 변수(WONJANG_NOTION_TOKEN)로만 받는다.
//!
//! 통합 만들기: notion.so/my-integrations → Internal Integration → 토큰 복사 →
//! 대상 페이지/DB의 '연결(Connections)'에 그 통합을 추가해야 접근됩니다.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

const API: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2022-06-28";

/// 검색 결과 한 건.
pub struct NotionHit {
    pub id: String,
    pub title: String,
    pub url: String,
    pub kind: String, // "page" | "database"
}

fn client(token: &str) -> Result<reqwest::Client> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).context("토큰 형식 오류")?,
    );
    headers.insert("Notion-Version", HeaderValue::from_static(NOTION_VERSION));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .context("HTTP 클라이언트 생성 실패")
}

/// 워크스페이스에서 페이지/DB를 검색한다.
pub async fn search(token: &str, query: &str, limit: usize) -> Result<Vec<NotionHit>> {
    let http = client(token)?;
    let resp = http
        .post(format!("{API}/search"))
        .json(&json!({ "query": query, "page_size": limit }))
        .send()
        .await
        .context("노션 검색 요청 실패")?;
    let body = check_ok(resp).await?;
    let mut hits = Vec::new();
    if let Some(arr) = body.get("results").and_then(|v| v.as_array()) {
        for item in arr.iter().take(limit) {
            let kind = item
                .get("object")
                .and_then(|v| v.as_str())
                .unwrap_or("page")
                .to_string();
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let title = extract_title(item);
            hits.push(NotionHit {
                id,
                title,
                url,
                kind,
            });
        }
    }
    Ok(hits)
}

/// 페이지에 단락(문단)을 덧붙인다.
pub async fn append_paragraph(token: &str, page_id: &str, text: &str) -> Result<()> {
    let http = client(token)?;
    let resp = http
        .patch(format!("{API}/blocks/{page_id}/children"))
        .json(&json!({
            "children": [{
                "object": "block",
                "type": "paragraph",
                "paragraph": {
                    "rich_text": [{ "type": "text", "text": { "content": text } }]
                }
            }]
        }))
        .send()
        .await
        .context("노션 덧붙이기 요청 실패")?;
    check_ok(resp).await?;
    Ok(())
}

/// 응답 상태를 확인하고 JSON 본문을 반환(오류 메시지 친화적으로).
async fn check_ok(resp: reqwest::Response) -> Result<Value> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        // 노션 오류 메시지 추출.
        let msg = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from))
            .unwrap_or_else(|| text.chars().take(200).collect());
        bail!("노션 API 오류({status}): {msg}");
    }
    serde_json::from_str(&text).context("노션 응답 파싱 실패")
}

/// 페이지/DB 객체에서 제목을 best-effort로 추출한다.
fn extract_title(item: &Value) -> String {
    // 데이터베이스: title 배열이 최상위.
    if let Some(t) = item.get("title").and_then(plain_from_rich) {
        if !t.is_empty() {
            return t;
        }
    }
    // 페이지: properties 안에서 type == "title" 인 속성을 찾는다.
    if let Some(props) = item.get("properties").and_then(|v| v.as_object()) {
        for prop in props.values() {
            if prop.get("type").and_then(|v| v.as_str()) == Some("title") {
                if let Some(t) = prop.get("title").and_then(plain_from_rich) {
                    if !t.is_empty() {
                        return t;
                    }
                }
            }
        }
    }
    "(제목 없음)".to_string()
}

/// rich_text/title 배열에서 plain_text를 이어붙인다.
fn plain_from_rich(v: &Value) -> Option<String> {
    let arr = v.as_array()?;
    let s: String = arr
        .iter()
        .filter_map(|x| x.get("plain_text").and_then(|p| p.as_str()))
        .collect();
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_title_from_page_properties() {
        let item = json!({
            "object": "page",
            "properties": {
                "Name": {
                    "type": "title",
                    "title": [{ "plain_text": "회의 노트" }]
                }
            }
        });
        assert_eq!(extract_title(&item), "회의 노트");
    }

    #[test]
    fn extract_title_from_database() {
        let item = json!({
            "object": "database",
            "title": [{ "plain_text": "할 일 DB" }]
        });
        assert_eq!(extract_title(&item), "할 일 DB");
    }

    #[test]
    fn extract_title_fallback() {
        let item = json!({ "object": "page", "properties": {} });
        assert_eq!(extract_title(&item), "(제목 없음)");
    }
}
