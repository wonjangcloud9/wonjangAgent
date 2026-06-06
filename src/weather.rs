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
    pub icon: String,
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
    #[serde(default)]
    is_day: i64,
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

/// 하루치 예보(주간 표시용).
pub struct DayForecast {
    pub date: String, // YYYY-MM-DD
    pub icon: String,
    pub desc: String,
    pub min: f64,
    pub max: f64,
    pub precip_prob: i64,
}

#[derive(Deserialize)]
struct WeeklyResp {
    daily: WeeklyDaily,
}

#[derive(Deserialize)]
struct WeeklyDaily {
    time: Vec<String>,
    weather_code: Vec<i64>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    #[serde(default)]
    precipitation_probability_max: Vec<i64>,
}

/// 7일 주간 예보(강수확률 포함). 반환: (지역명, 날짜순 예보).
pub async fn weekly(location: &str) -> Result<(String, Vec<DayForecast>)> {
    let (lat, lon, place) = resolve(location.trim()).await?;
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max\
         &timezone=Asia%2FSeoul&forecast_days=7"
    );
    let r: WeeklyResp = client()?
        .get(&url)
        .send()
        .await
        .context("주간 날씨 요청 실패")?
        .json()
        .await
        .context("주간 날씨 응답 파싱 실패")?;
    let d = r.daily;
    let days = (0..d.time.len())
        .map(|i| {
            let code = *d.weather_code.get(i).unwrap_or(&0);
            DayForecast {
                date: d.time[i].clone(),
                icon: wmo_emoji(code, true).to_string(),
                desc: wmo_desc(code).to_string(),
                min: *d.temperature_2m_min.get(i).unwrap_or(&0.0),
                max: *d.temperature_2m_max.get(i).unwrap_or(&0.0),
                precip_prob: d.precipitation_probability_max.get(i).copied().unwrap_or(0),
            }
        })
        .collect();
    Ok((place, days))
}

/// 지역 이름으로 날씨를 가져온다(비면 서울).
pub async fn weather(location: &str) -> Result<Weather> {
    let loc = location.trim();
    let (lat, lon, place) = resolve(loc).await?;

    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,precipitation,is_day\
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
        icon: wmo_emoji(f.current.weather_code, f.current.is_day == 1).to_string(),
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
    let loc = loc.trim();
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
    // 구어체 접미사 제거(긴 것 먼저, 단일 글자 마지막): '서울시·대전시·세종특별자치시' 등.
    let n = name
        .trim()
        .trim_end_matches("특별자치시")
        .trim_end_matches("특별자치도")
        .trim_end_matches("특별시")
        .trim_end_matches("광역시")
        .trim_end_matches('시')
        .trim_end_matches('도')
        .trim();
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

/// WMO 날씨 코드 → 이모지(상태에 맞게, 맑음·약한구름은 주야 반영).
fn wmo_emoji(code: i64, is_day: bool) -> &'static str {
    match code {
        0 => {
            if is_day {
                "☀️"
            } else {
                "🌙"
            }
        }
        1 => {
            if is_day {
                "🌤️"
            } else {
                "🌙"
            }
        }
        2 => "⛅",
        3 => "☁️",
        45 | 48 => "🌫️",
        51 | 53 | 55 | 56 | 57 => "🌦️",
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => "🌧️",
        71 | 73 | 75 | 77 | 85 | 86 => "🌨️",
        95 | 96 | 99 => "⛈️",
        _ => "🌡️",
    }
}

/// 체감온도(°C)별 옷차림 추천 — 기상청 기온별 옷차림 기준.
pub fn outfit(feels: f64) -> &'static str {
    let t = feels.round() as i64;
    match t {
        28.. => "민소매·반팔·반바지 🥵",
        23..=27 => "반팔·얇은 셔츠·면바지 👕",
        20..=22 => "긴팔·얇은 가디건·청바지 🙂",
        17..=19 => "맨투맨·니트·가디건 🧥",
        12..=16 => "자켓·야상·청바지 🧥",
        9..=11 => "트렌치코트·점퍼·니트 🧣",
        5..=8 => "코트·가죽자켓·히트텍 🧥",
        _ => "두꺼운 패딩·목도리·기모 🥶",
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

    #[test]
    fn wmo_emoji_matches_condition() {
        // 상태에 맞는 이모지(비/눈/흐림이 ☀️로 안 나와야).
        assert_eq!(wmo_emoji(0, true), "☀️"); // 맑음 낮
        assert_eq!(wmo_emoji(0, false), "🌙"); // 맑음 밤
        assert_eq!(wmo_emoji(3, true), "☁️"); // 흐림
        assert_eq!(wmo_emoji(61, true), "🌧️"); // 비
        assert_eq!(wmo_emoji(71, true), "🌨️"); // 눈
        assert_eq!(wmo_emoji(95, true), "⛈️"); // 뇌우
                                               // 비/눈은 낮밤 무관하게 ☀️가 아님.
        assert_ne!(wmo_emoji(61, true), "☀️");
        assert_ne!(wmo_emoji(3, false), "☀️");
    }

    #[test]
    fn outfit_by_feels_temp() {
        assert!(outfit(30.0).contains("반팔"));
        assert!(outfit(25.0).contains("반팔"));
        assert!(outfit(18.0).contains("니트") || outfit(18.0).contains("맨투맨"));
        assert!(outfit(2.0).contains("패딩"));
        assert!(outfit(7.0).contains("코트"));
    }

    #[tokio::test]
    #[ignore]
    async fn live_seoul() {
        let w = weather("서울").await.unwrap();
        assert!(w.temp > -50.0 && w.temp < 60.0);
        assert!(!w.desc.is_empty());
    }
}
