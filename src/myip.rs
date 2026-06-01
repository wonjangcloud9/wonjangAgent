//! 내 공인 IP·통신사·위치 — "내 IP 뭐지? VPN 켜졌나?"를 즉시 확인한다.
//!
//! ip-api.com(무료, 키 불필요)으로 현재 나가는 공인 IP와 그에 매핑된 국가·지역·
//! 통신사를 조회한다. 내 네트워크 환경 정보라 GPT로는 알 수 없다.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Resp {
    #[serde(default)]
    status: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    country: String,
    #[serde(rename = "regionName", default)]
    region: String,
    #[serde(default)]
    city: String,
    #[serde(default)]
    isp: String,
    #[serde(default)]
    org: String,
}

/// 공인 IP 정보.
pub struct IpInfo {
    pub ip: String,
    pub country: String,
    pub region: String,
    pub city: String,
    pub isp: String,
    pub org: String,
}

/// 현재 공인 IP와 위치·통신사를 조회한다.
pub async fn fetch() -> Result<IpInfo> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let r: Resp = http
        .get("http://ip-api.com/json/?fields=status,query,country,regionName,city,isp,org&lang=ko")
        .send()
        .await
        .context("IP 조회 요청 실패")?
        .json()
        .await
        .context("IP 조회 응답 파싱 실패")?;
    if r.status != "success" || r.query.is_empty() {
        bail!("공인 IP를 가져오지 못했습니다");
    }
    Ok(IpInfo {
        ip: r.query,
        country: r.country,
        region: r.region,
        city: r.city,
        isp: r.isp,
        org: r.org,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_response() {
        let json = r#"{"status":"success","query":"1.2.3.4","country":"대한민국","regionName":"인천","city":"부평구","isp":"LG","org":""}"#;
        let r: Resp = serde_json::from_str(json).unwrap();
        assert_eq!(r.status, "success");
        assert_eq!(r.query, "1.2.3.4");
        assert_eq!(r.city, "부평구");
    }
}
