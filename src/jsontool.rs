//! JSON 도구 — 로컬 JSON 파일을 검증·정렬(pretty)하고 값을 추출한다.
//!
//! "이 설정 파일 JSON 맞아?", "여기서 version 값 뭐야?" 같은 요청을 처리한다.
//! serde_json만 쓰며 네트워크·키가 필요 없다.

use anyhow::{anyhow, Result};
use serde_json::Value;

/// 파싱 결과 요약(타입과 크기).
pub fn summary(v: &Value) -> String {
    match v {
        Value::Object(m) => format!("객체 {{{}개 키}}", m.len()),
        Value::Array(a) => format!("배열 [{}개 항목]", a.len()),
        Value::String(s) => format!("문자열 ({}자)", s.chars().count()),
        Value::Number(n) => format!("숫자 {n}"),
        Value::Bool(b) => format!("불리언 {b}"),
        Value::Null => "null".to_string(),
    }
}

/// 점 경로(`a.b.0.c`)로 값을 찾는다. 배열은 숫자 인덱스로.
pub fn pick<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = root;
    for seg in path.split('.') {
        if seg.is_empty() {
            continue;
        }
        cur = match cur {
            Value::Object(m) => m.get(seg)?,
            Value::Array(a) => a.get(seg.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// 파일을 읽어 파싱한다. 오류면 위치 정보를 담은 메시지.
pub fn parse_file(path: &str) -> Result<Value> {
    // 파일 I/O 에러도 한국어로(다른 파일 명령과 일관 — 영문 OS 에러 누출 방지).
    let text = std::fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => anyhow!("파일을 찾을 수 없어요: {path}"),
        _ => anyhow!("파일을 읽을 수 없어요: {path}"),
    })?;
    parse_str(&text)
}

/// 문자열을 JSON으로 파싱한다. 형식 오류는 위치(행·열)와 함께 한국어로 안내한다
/// (영문 serde 메시지는 붙이지 않는다 — 한국어 우선).
pub fn parse_str(text: &str) -> Result<Value> {
    serde_json::from_str(text).map_err(|e| {
        anyhow!(
            "JSON 형식이 올바르지 않아요 ({}행 {}열 근처)",
            e.line(),
            e.column()
        )
    })
}

/// 값을 보기 좋게(들여쓰기) 직렬화.
pub fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        serde_json::json!({
            "name": "원장",
            "version": "0.64.2",
            "tags": ["a", "b", "c"],
            "meta": {"port": 8080}
        })
    }

    #[test]
    fn errors_are_korean_without_english() {
        // 형식 오류: 한국어 + 위치, 영문 serde 잔재 없음.
        let err = parse_str("not,valid").unwrap_err().to_string();
        assert!(err.contains("올바르지 않아요"), "{err}");
        assert!(!err.to_lowercase().contains("expected"), "영문 누출: {err}");
        // 없는 파일: 영문 OS 에러 대신 한국어.
        let err = parse_file("definitely/no/such/file.json")
            .unwrap_err()
            .to_string();
        assert!(err.contains("찾을 수 없어요"), "{err}");
        assert!(!err.contains("os error"), "영문 누출: {err}");
        // 정상 파싱은 그대로.
        assert!(parse_str("{\"a\":1}").is_ok());
    }

    #[test]
    fn summarizes_types() {
        let v = sample();
        assert_eq!(summary(&v), "객체 {4개 키}");
        assert_eq!(summary(&v["tags"]), "배열 [3개 항목]");
    }

    #[test]
    fn picks_by_dot_path() {
        let v = sample();
        assert_eq!(pick(&v, "version").unwrap(), "0.64.2");
        assert_eq!(pick(&v, "meta.port").unwrap(), 8080);
        assert_eq!(pick(&v, "tags.1").unwrap(), "b");
        assert!(pick(&v, "없는.경로").is_none());
    }

    #[test]
    fn pretty_is_indented() {
        let p = pretty(&sample());
        assert!(p.contains("\n  \""));
    }
}
