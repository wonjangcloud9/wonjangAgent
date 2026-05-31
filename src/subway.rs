//! 서울 지하철 실시간 도착정보 — 역 이름만으로 도착 시간을 가져온다.
//!
//! 서울 열린데이터광장의 실시간 도착 API를 쓴다. 역 ID가 필요 없고 역 이름만으로
//! 조회되며, 'sample' 키로도 테스트가 된다(실사용은 data.seoul.go.kr에서 키 발급).

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// 도착 정보 한 건.
pub struct Arrival {
    pub line: String,      // 호선(예: 2호선, 신분당선)
    pub direction: String, // 방면(trainLineNm)
    pub message: String,   // 도착 안내(arvlMsg2, 예: "2분 30초 후")
}

#[derive(Deserialize)]
struct ApiResp {
    #[serde(rename = "realtimeArrivalList", default)]
    arrivals: Vec<RawArrival>,
    #[serde(rename = "errorMessage", default)]
    error: Option<ErrorMessage>,
}

#[derive(Deserialize)]
struct ErrorMessage {
    #[serde(default)]
    status: i64,
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct RawArrival {
    #[serde(rename = "subwayId", default)]
    subway_id: String,
    #[serde(rename = "trainLineNm", default)]
    train_line: String,
    #[serde(rename = "arvlMsg2", default)]
    arvl_msg2: String,
}

/// 역 이름으로 실시간 도착정보를 조회한다.
pub async fn arrivals(api_key: &str, station: &str, limit: usize) -> Result<Vec<Arrival>> {
    let station = station.trim().trim_end_matches('역'); // '강남역' → '강남'
                                                         // 샘플 키는 최대 5건 제한.
    let limit = if api_key == "sample" {
        limit.min(5)
    } else {
        limit.clamp(1, 20)
    };
    let url = format!(
        "http://swopenapi.seoul.go.kr/api/subway/{}/json/realtimeStationArrival/0/{}/{}",
        api_key,
        limit,
        encode(station)
    );
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let text = http
        .get(&url)
        .send()
        .await
        .context("지하철 도착정보 요청 실패")?
        .text()
        .await
        .context("응답 읽기 실패")?;

    let resp: ApiResp =
        serde_json::from_str(&text).with_context(|| "지하철 응답 파싱 실패".to_string())?;

    if resp.arrivals.is_empty() {
        if let Some(e) = resp.error {
            // INFO-200 = 데이터 없음(역명 오타 등). status 200이면 그냥 빈 결과.
            if e.status != 200 && !e.message.is_empty() {
                bail!("{}", e.message);
            }
        }
    }

    Ok(resp
        .arrivals
        .into_iter()
        .map(|r| Arrival {
            line: line_name(&r.subway_id),
            direction: r.train_line,
            message: r.arvl_msg2,
        })
        .collect())
}

/// subwayId 코드를 호선 이름으로.
fn line_name(id: &str) -> String {
    match id {
        "1001" => "1호선",
        "1002" => "2호선",
        "1003" => "3호선",
        "1004" => "4호선",
        "1005" => "5호선",
        "1006" => "6호선",
        "1007" => "7호선",
        "1008" => "8호선",
        "1009" => "9호선",
        "1063" => "경의중앙",
        "1065" => "공항철도",
        "1067" => "경춘",
        "1075" => "수인분당",
        "1077" => "신분당",
        "1092" => "우이신설",
        "1093" => "서해",
        "1081" => "경강",
        "1032" => "GTX-A",
        _ => "지하철",
    }
    .to_string()
}

/// 경로용 퍼센트 인코딩(UTF-8).
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
    fn line_names() {
        assert_eq!(line_name("1002"), "2호선");
        assert_eq!(line_name("1077"), "신분당");
        assert_eq!(line_name("9999"), "지하철");
    }

    #[test]
    fn encode_korean() {
        assert_eq!(encode("강남"), "%EA%B0%95%EB%82%A8");
        assert_eq!(encode("Seoul"), "Seoul");
    }

    // 네트워크 라이브 테스트(sample 키). 실행: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn live_gangnam() {
        let res = arrivals("sample", "강남", 5).await.unwrap();
        assert!(!res.is_empty(), "강남역 도착정보가 비어 있음");
        assert!(res[0].line.contains("호선") || res[0].line.contains("분당"));
    }
}
