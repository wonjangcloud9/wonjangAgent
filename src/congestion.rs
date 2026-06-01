//! 서울 실시간 혼잡도 — "지금 거기 사람 많아?"를 알려준다.
//!
//! 서울 열린데이터광장의 실시간 도시데이터(citydata_ppltn)를 사용한다. 주요
//! 명소·상권·지하철역 등 약 120곳의 실시간 혼잡도 단계와 추정 인구를 준다.
//! `sample` 키로는 고정 예시(광화문·덕수궁)만 나오며, data.seoul.go.kr에서
//! 무료 키를 발급받아 설정하면 원하는 지역을 조회할 수 있다.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Resp {
    #[serde(rename = "SeoulRtd.citydata_ppltn", default)]
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    #[serde(rename = "AREA_NM", default)]
    area: String,
    #[serde(rename = "AREA_CONGEST_LVL", default)]
    level: String,
    #[serde(rename = "AREA_CONGEST_MSG", default)]
    message: String,
    #[serde(rename = "AREA_PPLTN_MIN", default)]
    ppltn_min: String,
    #[serde(rename = "AREA_PPLTN_MAX", default)]
    ppltn_max: String,
    #[serde(rename = "PPLTN_TIME", default)]
    time: String,
}

/// 한 지역의 실시간 혼잡도.
pub struct Congestion {
    pub area: String,
    pub level: String,
    pub message: String,
    pub ppltn_min: String,
    pub ppltn_max: String,
    pub time: String,
    pub is_sample: bool,
}

/// 지역 이름으로 실시간 혼잡도를 조회한다.
pub async fn fetch(api_key: &str, area: &str) -> Result<Congestion> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    // 지역 이름은 경로에 넣는다(서울 OpenAPI 규격).
    let url = format!(
        "http://openapi.seoul.go.kr:8088/{}/json/citydata_ppltn/1/5/{}",
        api_key, area
    );
    let r: Resp = http
        .get(&url)
        .send()
        .await
        .context("혼잡도 요청 실패")?
        .json()
        .await
        .context("혼잡도 응답 파싱 실패")?;
    let row = r
        .rows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("'{area}' 지역 데이터를 찾지 못했어요"))?;
    if row.area.is_empty() {
        bail!("혼잡도 데이터가 비어 있어요");
    }
    Ok(Congestion {
        is_sample: api_key == "sample",
        area: row.area,
        level: row.level,
        message: row.message,
        ppltn_min: row.ppltn_min,
        ppltn_max: row.ppltn_max,
        time: row.time,
    })
}

/// 혼잡도 단계에 어울리는 이모지.
pub fn level_emoji(level: &str) -> &'static str {
    match level {
        "붐빔" => "🔴",
        "약간 붐빔" => "🟠",
        "보통" => "🟡",
        "여유" => "🟢",
        _ => "⚪",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_mapping() {
        assert_eq!(level_emoji("붐빔"), "🔴");
        assert_eq!(level_emoji("여유"), "🟢");
        assert_eq!(level_emoji("알수없음"), "⚪");
    }
}
