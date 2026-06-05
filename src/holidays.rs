//! 한국 공휴일 — "다음 빨간날 언제?"·연차 계획에 쓰는 실시간 조회(무료, 키 불필요).
//!
//! Nager.Date 공개 API에서 한국 공휴일을 받는다. 설날·추석처럼 음력 기반
//! 공휴일도 API가 정확한 날짜로 내려주므로 직접 음력을 계산하지 않아도 된다.

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Weekday};
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

/// 실제로 '쉬는 날'인 공휴일 날짜 집합.
///
/// 제헌절은 국경일이지만 2008년부터 쉬는 날이 아니다 — 연휴 계산에서 제외해야
/// 가짜 연휴를 만들지 않는다(Nager API는 다른 공휴일과 똑같이 Public으로 줌).
fn off_holiday_set(holidays: &[Holiday]) -> std::collections::HashSet<NaiveDate> {
    holidays
        .iter()
        .filter(|h| h.name != "제헌절")
        .map(|h| h.date)
        .collect()
}

/// 오늘 이후 첫 '연휴'(주말+공휴일이 연속으로 이어지는 쉬는 구간) 중 `min_len`일 이상인 것.
///
/// 반환: `(시작일, 종료일, 연속 일수)`. "이번 추석 며칠 쉬어?"를 한 줄로 답한다 —
/// GPT가 자주 틀리는 대체공휴일·주말 끼임을 결정론적으로 계산. 데이터가 그 해 공휴일이라
/// 같은 해(12-31) 안에서만 본다. 쉬는 날 = 토·일 또는 공휴일.
pub fn next_long_break(
    holidays: &[Holiday],
    today: NaiveDate,
    min_len: usize,
) -> Option<(NaiveDate, NaiveDate, usize)> {
    let hset = off_holiday_set(holidays);
    let is_off =
        |d: NaiveDate| matches!(d.weekday(), Weekday::Sat | Weekday::Sun) || hset.contains(&d);
    let year_end = NaiveDate::from_ymd_opt(today.year(), 12, 31)?;
    let mut d = today;
    while d <= year_end {
        if !is_off(d) {
            d = d.succ_opt()?;
            continue;
        }
        let start = d;
        let mut end = d;
        while let Some(n) = end.succ_opt() {
            if n <= year_end && is_off(n) {
                end = n;
            } else {
                break;
            }
        }
        let len = (end - start).num_days() as usize + 1;
        if len >= min_len {
            return Some((start, end, len));
        }
        d = end.succ_opt()?;
    }
    None
}

/// '연차 하나'로 만드는 황금연휴 후보(징검다리). 평일 하루를 쉬면 앞뒤 휴일과 이어져
/// `min_break`일 이상 연속으로 쉬는 경우만 — "연차 하루로 4일 쉬기" 꿀팁. 한국에서
/// 연말마다 가장 많이 공유되는 정보로, GPT가 자주 틀리는 대체공휴일·요일 끼임을 정확히 계산.
pub struct GoldenLeave {
    pub leave: NaiveDate, // 연차 쓸 평일
    pub start: NaiveDate, // 연휴 시작
    pub end: NaiveDate,   // 연휴 끝
    pub len: usize,       // 연속 일수
}

/// 오늘 이후 황금연휴 후보를 긴 연휴 순으로 최대 `limit`개(날짜순 반환).
pub fn golden_leaves(
    holidays: &[Holiday],
    today: NaiveDate,
    min_break: usize,
    limit: usize,
) -> Vec<GoldenLeave> {
    let hset = off_holiday_set(holidays);
    let is_off =
        |d: NaiveDate| matches!(d.weekday(), Weekday::Sat | Weekday::Sun) || hset.contains(&d);
    let Some(year_end) = NaiveDate::from_ymd_opt(today.year(), 12, 31) else {
        return Vec::new();
    };
    let mut out: Vec<GoldenLeave> = Vec::new();
    let mut d = today;
    while d <= year_end {
        if !is_off(d) {
            // 왼쪽 연속 휴일(지난 날은 이미 쉬었으니 오늘 이후만).
            let mut start = d;
            while let Some(p) = start.pred_opt() {
                if p >= today && is_off(p) {
                    start = p;
                } else {
                    break;
                }
            }
            // 오른쪽 연속 휴일(연말까지).
            let mut end = d;
            while let Some(q) = end.succ_opt() {
                if q <= year_end && is_off(q) {
                    end = q;
                } else {
                    break;
                }
            }
            let len = (end - start).num_days() as usize + 1;
            // 양쪽 중 하나라도 휴일에 붙어야 황금연휴(고립 평일 제외).
            if len >= min_break && (start < d || end > d) {
                out.push(GoldenLeave {
                    leave: d,
                    start,
                    end,
                    len,
                });
            }
        }
        match d.succ_opt() {
            Some(n) => d = n,
            None => break,
        }
    }
    out.sort_by(|a, b| b.len.cmp(&a.len).then(a.leave.cmp(&b.leave)));
    out.truncate(limit);
    out.sort_by_key(|g| g.leave);
    out
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

    fn hol(dates: &[(u32, u32)]) -> Vec<Holiday> {
        dates
            .iter()
            .map(|&(m, day)| Holiday {
                date: d(2026, m, day),
                name: "휴일".into(),
            })
            .collect()
    }

    #[test]
    fn long_break_chuseok_2026() {
        // 2026 추석 09-24(목)~09-26(토) + 일(09-27) = 목금토일 4일 연휴.
        let hs = hol(&[(9, 24), (9, 25), (9, 26)]);
        let (start, end, len) = next_long_break(&hs, d(2026, 9, 1), 3).unwrap();
        assert_eq!(start, d(2026, 9, 24)); // 목
        assert_eq!(end, d(2026, 9, 27)); // 일
        assert_eq!(len, 4);
    }

    #[test]
    fn long_break_skips_plain_weekend() {
        // 공휴일 없이 토·일(2일)만 있으면 3일 연휴 조건 미달 → 다음(월 공휴일 낀 3일) 선택.
        let hs = hol(&[(6, 8)]); // 2026-06-08은 월요일
        let (start, end, len) = next_long_break(&hs, d(2026, 6, 1), 3).unwrap();
        assert_eq!(start, d(2026, 6, 6)); // 토
        assert_eq!(end, d(2026, 6, 8)); // 월(공휴일)
        assert_eq!(len, 3);
    }

    #[test]
    fn long_break_none_when_only_weekends() {
        // 평일 단독 공휴일(화)뿐이면 주말(2일)만 남아 3일 연휴 없음.
        let hs = hol(&[(7, 14)]); // 2026-07-14는 화요일(주말과 안 붙음)
        assert!(next_long_break(&hs, d(2026, 7, 13), 3).is_none());
    }

    #[test]
    fn long_break_excludes_constitution_day() {
        // 제헌절(07-17 금)은 쉬는 날이 아니다 → 금+토일로 가짜 3일 연휴를 만들면 안 됨.
        let hs = vec![Holiday {
            date: d(2026, 7, 17),
            name: "제헌절".into(),
        }];
        assert!(next_long_break(&hs, d(2026, 7, 1), 3).is_none());
    }

    #[test]
    fn golden_leave_bridges_holiday_and_weekend() {
        // 08-15(토)~08-17(월 광복절 대체) 휴일. 08-18(화) 연차 → 토일월화 4일.
        let hs = hol(&[(8, 17)]);
        let g = golden_leaves(&hs, d(2026, 8, 1), 4, 5);
        let tue = g
            .iter()
            .find(|x| x.leave == d(2026, 8, 18))
            .expect("08-18 화 연차 4일");
        assert_eq!(tue.start, d(2026, 8, 15));
        assert_eq!(tue.end, d(2026, 8, 18));
        assert_eq!(tue.len, 4);
        // 반대편 08-14(금) 연차도 금토일월 4일.
        assert!(g.iter().any(|x| x.leave == d(2026, 8, 14) && x.len == 4));
    }

    #[test]
    fn golden_leave_skips_when_no_holiday_nearby() {
        // 공휴일 없으면 연차 1개 + 주말 = 최대 3일 → 4일 황금연휴 없음.
        assert!(golden_leaves(&[], d(2026, 6, 1), 4, 5).is_empty());
    }
}
