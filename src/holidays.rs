//! 한국 공휴일 — "다음 빨간날 언제?"·연차 계획에 쓰는 실시간 조회(무료, 키 불필요).
//!
//! Nager.Date 공개 API에서 한국 공휴일을 받는다. 설날·추석처럼 음력 기반
//! 공휴일도 API가 정확한 날짜로 내려주므로 직접 음력을 계산하지 않아도 된다.

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Deserialize)]
struct Raw {
    date: String,
    #[serde(rename = "localName")]
    local_name: String,
}

/// 공휴일 한 건.
pub struct Holiday {
    pub date: NaiveDate,
    pub name: String,
}

/// 해당 연도의 한국 공휴일을 날짜순으로 가져온다.
pub async fn fetch(year: i32) -> Result<Vec<Holiday>> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let url = format!("https://date.nager.at/api/v3/PublicHolidays/{year}/KR");
    let raws: Vec<Raw> = http
        .get(&url)
        .send()
        .await
        .context("공휴일 요청 실패")?
        .json()
        .await
        .context("공휴일 응답 파싱 실패")?;

    let mut out: Vec<Holiday> = raws
        .into_iter()
        .filter_map(|r| {
            NaiveDate::parse_from_str(&r.date, "%Y-%m-%d")
                .ok()
                .map(|date| Holiday {
                    date,
                    name: r.local_name,
                })
        })
        .collect();
    out.sort_by_key(|h| h.date);
    Ok(out)
}

/// 같은 날짜에 이름이 여러 개면(연휴) 하나로 묶어 보기 좋게.
pub fn next_after(holidays: &[Holiday], today: NaiveDate) -> Option<&Holiday> {
    holidays.iter().find(|h| h.date >= today)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn next_after_picks_first_future() {
        let hs = vec![
            Holiday {
                date: d(2026, 1, 1),
                name: "새해".into(),
            },
            Holiday {
                date: d(2026, 3, 1),
                name: "삼일절".into(),
            },
        ];
        assert_eq!(next_after(&hs, d(2026, 1, 2)).unwrap().name, "삼일절");
        assert_eq!(next_after(&hs, d(2026, 1, 1)).unwrap().name, "새해");
        assert!(next_after(&hs, d(2026, 12, 1)).is_none());
    }
}
