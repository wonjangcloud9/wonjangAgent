//! 날씨 — 무료 open-meteo API로 실시간 날씨를 가져온다(키 불필요).
//!
//! 주요 한국 도시는 좌표를 내장해 정확하게, 그 외 지역은 지오코딩으로 찾는다.
//! 기존 `날씨` 프리셋(web_search 기반)보다 정확한 구조화 데이터를 제공한다.

use anyhow::{Context, Result};
use serde::Deserialize;

pub struct Weather {
    pub place: String,
    pub temp: f64,
    pub feels: f64,
    pub humidity: i64,
    pub desc: String,
    pub today_min: f64,
    pub today_max: f64,
    pub precip: f64,
}

#[derive(Deserialize)]
struct Forecast {
    current: Current,
    daily: Daily,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f64,
    relative_humidity_2m: i64,
    apparent_temperature: f64,
    weather_code: i64,
    precipitation: f64,
}

#[derive(Deserialize)]
struct Daily {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

#[derive(Deserialize)]
struct GeoResp {
    #[serde(default)]
    results: Vec<GeoHit>,
}

#[derive(Deserialize)]
struct GeoHit {
    name: String,
    latitude: f64,
    longitude: f64,
}

/// 지역 이름으로 날씨를 가져온다(비면 서울).
pub async fn weather(location: &str) -> Result<Weather> {
    let loc = location.trim();
    let (lat, lon, place) = resolve(loc).await?;

    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,precipitation\
         &daily=temperature_2m_max,temperature_2m_min&timezone=Asia%2FSeoul&forecast_days=1"
    );
    let http = client()?;
    let f: Forecast = http
        .get(&url)
        .send()
        .await
        .context("날씨 요청 실패")?
        .json()
        .await
        .context("날씨 응답 파싱 실패")?;

    Ok(Weather {
        place,
        temp: f.current.temperature_2m,
        feels: f.current.apparent_temperature,
        humidity: f.current.relative_humidity_2m,
        desc: wmo_desc(f.current.weather_code).to_string(),
        today_min: f.daily.temperature_2m_min.first().copied().unwrap_or(0.0),
        today_max: f.daily.temperature_2m_max.first().copied().unwrap_or(0.0),
        precip: f.current.precipitation,
    })
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")
}

/// 지역 이름 → (위도, 경도, 표시이름). 주요 도시는 내장, 그 외는 지오코딩.
/// (대기질 등 다른 위치 기반 기능에서도 재사용한다.)
pub async fn resolve(loc: &str) -> Result<(f64, f64, String)> {
    if loc.is_empty() {
        return Ok((37.5665, 126.9780, "서울".to_string()));
    }
    if let Some((lat, lon)) = korean_city(loc) {
        return Ok((lat, lon, loc.to_string()));
    }
    // 지오코딩 폴백.
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=ko",
        encode(loc)
    );
    let resp: GeoResp = client()?
        .get(&url)
        .send()
        .await
        .context("지오코딩 요청 실패")?
        .json()
        .await
        .context("지오코딩 응답 파싱 실패")?;
    match resp.results.into_iter().next() {
        Some(h) => Ok((h.latitude, h.longitude, h.name)),
        None => anyhow::bail!("'{loc}' 위치를 찾지 못했어요. 도시 이름을 확인해 주세요."),
    }
}

/// 주요 한국 도시 좌표(지오코딩보다 정확).
fn korean_city(name: &str) -> Option<(f64, f64)> {
    let n = name.trim_end_matches("특별시").trim_end_matches("광역시");
    let coord = match n {
        "서울" => (37.5665, 126.9780),
        "부산" => (35.1796, 129.0756),
        "대구" => (35.8714, 128.6014),
        "인천" => (37.4563, 126.7052),
        "광주" => (35.1595, 126.8526),
        "대전" => (36.3504, 127.3845),
        "울산" => (35.5384, 129.3114),
        "세종" => (36.4800, 127.2890),
        "수원" => (37.2636, 127.0286),
        "성남" => (37.4200, 127.1267),
        "제주" => (33.4996, 126.5312),
        "춘천" => (37.8813, 127.7300),
        "강릉" => (37.7519, 128.8761),
        "전주" => (35.8242, 127.1480),
        "청주" => (36.6424, 127.4890),
        "포항" => (36.0190, 129.3435),
        "창원" => (35.2281, 128.6811),
        _ => return None,
    };
    Some(coord)
}

/// WMO 날씨 코드 → 한국어 설명.
fn wmo_desc(code: i64) -> &'static str {
    match code {
        0 => "맑음",
        1 => "대체로 맑음",
        2 => "구름 조금",
        3 => "흐림",
        45 | 48 => "안개",
        51 | 53 | 55 => "이슬비",
        56..=57 => "어는 이슬비",
        61 | 63 | 65 => "비",
        66..=67 => "진눈깨비",
        71 | 73 | 75 => "눈",
        77 => "싸락눈",
        80..=82 => "소나기",
        85..=86 => "소나기눈",
        95 => "뇌우",
        96 | 99 => "우박 동반 뇌우",
        _ => "알 수 없음",
    }
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
    fn city_lookup() {
        assert!(korean_city("서울").is_some());
        assert!(korean_city("서울특별시").is_some());
        assert!(korean_city("부산").is_some());
        assert!(korean_city("없는도시").is_none());
    }

    #[test]
    fn wmo_codes() {
        assert_eq!(wmo_desc(0), "맑음");
        assert_eq!(wmo_desc(61), "비");
        assert_eq!(wmo_desc(71), "눈");
    }

    #[tokio::test]
    #[ignore]
    async fn live_seoul() {
        let w = weather("서울").await.unwrap();
        assert!(w.temp > -50.0 && w.temp < 60.0);
        assert!(!w.desc.is_empty());
    }
}
