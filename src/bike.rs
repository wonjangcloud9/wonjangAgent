//! 서울 따릉이 실시간 — 대여소의 남은 자전거·거치대를 조회한다.
//!
//! 서울 열린데이터광장 `bikeList`를 쓴다. 대여소 이름으로 검색한다. `sample`
//! 키로는 고정 예시(망원역 일대)만 나오며, data.seoul.go.kr 무료 키를 넣으면
//! 전체 대여소를 조회할 수 있다. 지하철·혼잡도와 같은 키 슬롯을 공유한다.

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Resp {
    #[serde(rename = "rentBikeStatus", default)]
    status: Option<Status>,
}

#[derive(Deserialize)]
struct Status {
    #[serde(default)]
    row: Vec<RawStation>,
}

#[derive(Deserialize)]
struct RawStation {
    #[serde(rename = "stationName", default)]
    name: String,
    #[serde(rename = "parkingBikeTotCnt", default)]
    bikes: String,
    #[serde(rename = "rackTotCnt", default)]
    racks: String,
}

/// 대여소 한 곳.
pub struct Station {
    pub name: String,
    pub bikes: u32,
    pub racks: u32,
}

/// 대여소를 이름으로 검색한다. `query`가 비면 앞쪽 일부를 반환.
pub async fn fetch(api_key: &str, query: &str) -> Result<(Vec<Station>, bool)> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let is_sample = api_key == "sample";

    // 실키는 1~1000, 1001~2000, 2001~3000 페이지를 모은다(전체 ~2700곳).
    let ranges: &[(u32, u32)] = if is_sample {
        &[(1, 5)]
    } else {
        &[(1, 1000), (1001, 2000), (2001, 3000)]
    };

    let mut all: Vec<Station> = Vec::new();
    for (start, end) in ranges {
        let url = format!("http://openapi.seoul.go.kr:8088/{api_key}/json/bikeList/{start}/{end}/");
        let r: Resp = match http.get(&url).send().await {
            Ok(resp) => match resp.json().await {
                Ok(j) => j,
                Err(_) => continue,
            },
            Err(_) => continue,
        };
        if let Some(status) = r.status {
            for s in status.row {
                all.push(Station {
                    name: s.name,
                    bikes: s.bikes.parse().unwrap_or(0),
                    racks: s.racks.parse().unwrap_or(0),
                });
            }
        }
        // 더 받을 게 없으면 중단.
        if all.is_empty() {
            break;
        }
    }

    let q = query.trim();
    let filtered: Vec<Station> = if q.is_empty() {
        all.into_iter().take(15).collect()
    } else {
        all.into_iter()
            .filter(|s| s.name.contains(q))
            .take(20)
            .collect()
    };
    Ok((filtered, is_sample))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_station_numbers() {
        let s = RawStation {
            name: "102. 망원역 1번출구 앞".into(),
            bikes: "13".into(),
            racks: "15".into(),
        };
        let st = Station {
            name: s.name.clone(),
            bikes: s.bikes.parse().unwrap_or(0),
            racks: s.racks.parse().unwrap_or(0),
        };
        assert_eq!(st.bikes, 13);
        assert_eq!(st.racks, 15);
        assert!(st.name.contains("망원역"));
    }
}
