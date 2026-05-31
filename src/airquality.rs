//! 미세먼지(대기질) — 한국인이 매일 확인하는 PM10/PM2.5를 환경부 기준 등급과 함께.
//!
//! 무료 open-meteo 대기질 API(키 불필요). 위치 해석은 weather::resolve를 재사용한다.

use crate::weather;
use anyhow::{Context, Result};
use serde::Deserialize;

pub struct AirQuality {
    pub place: String,
    pub pm10: f64,
    pub pm25: f64,
}

#[derive(Deserialize)]
struct Resp {
    current: Current,
}

#[derive(Deserialize)]
struct Current {
    #[serde(default)]
    pm10: f64,
    #[serde(default)]
    pm2_5: f64,
}

/// 지역의 미세먼지를 가져온다(비면 서울).
pub async fn air_quality(location: &str) -> Result<AirQuality> {
    let (lat, lon, place) = weather::resolve(location).await?;
    let url = format!(
        "https://air-quality-api.open-meteo.com/v1/air-quality?latitude={lat}&longitude={lon}\
         &current=pm10,pm2_5&timezone=Asia%2FSeoul"
    );
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let r: Resp = http
        .get(&url)
        .send()
        .await
        .context("대기질 요청 실패")?
        .json()
        .await
        .context("대기질 응답 파싱 실패")?;
    Ok(AirQuality {
        place,
        pm10: r.current.pm10,
        pm25: r.current.pm2_5,
    })
}

/// 초미세먼지(PM2.5) 등급(환경부 기준).
pub fn grade_pm25(v: f64) -> (&'static str, &'static str) {
    match v as i64 {
        0..=15 => ("좋음", "😊"),
        16..=35 => ("보통", "🙂"),
        36..=75 => ("나쁨", "😷"),
        _ => ("매우나쁨", "🤢"),
    }
}

/// 미세먼지(PM10) 등급(환경부 기준).
pub fn grade_pm10(v: f64) -> (&'static str, &'static str) {
    match v as i64 {
        0..=30 => ("좋음", "😊"),
        31..=80 => ("보통", "🙂"),
        81..=150 => ("나쁨", "😷"),
        _ => ("매우나쁨", "🤢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades() {
        assert_eq!(grade_pm25(10.0).0, "좋음");
        assert_eq!(grade_pm25(46.0).0, "나쁨");
        assert_eq!(grade_pm25(90.0).0, "매우나쁨");
        assert_eq!(grade_pm10(20.0).0, "좋음");
        assert_eq!(grade_pm10(46.0).0, "보통");
        assert_eq!(grade_pm10(200.0).0, "매우나쁨");
    }

    #[tokio::test]
    #[ignore]
    async fn live_seoul() {
        let a = air_quality("서울").await.unwrap();
        assert!(a.pm10 >= 0.0 && a.pm25 >= 0.0);
    }
}
