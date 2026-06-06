//! 습관 트래커 — 매일 습관을 체크하고 연속 일수(streak)를 챙긴다.
//!
//! "갓생" 문화에 맞춘 자기계발 기능. 습관별로 완료한 날짜를 모으고, 오늘까지
//! 이어진 연속 일수를 계산해 동기를 준다. 브리핑에도 함께 보여준다.
//!
//! 저장 위치: `~/.local/share/wonjang/habits.json`

use anyhow::{Context, Result};
use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Habit {
    pub id: u64,
    pub name: String,
    /// 완료한 날짜들(YYYY-MM-DD).
    #[serde(default)]
    pub dates: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HabitStore {
    #[serde(default)]
    pub items: Vec<Habit>,
    #[serde(default)]
    next_id: u64,
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("habits.json"))
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn today_str() -> String {
    today().format("%Y-%m-%d").to_string()
}

/// 완료 날짜 집합 기준 연속 일수(streak)를 계산한다.
///
/// 오늘 완료했으면 오늘부터, 아직이면 어제부터 거꾸로 세어 끊기기 전까지.
/// (아직 오늘 체크 안 했어도 어제까지 이어졌으면 streak는 살아 있음.)
pub fn streak(dates: &HashSet<String>, today: NaiveDate) -> i64 {
    let fmt = |d: NaiveDate| d.format("%Y-%m-%d").to_string();
    let mut day = today;
    if !dates.contains(&fmt(today)) {
        day = today - Duration::days(1);
        if !dates.contains(&fmt(day)) {
            return 0;
        }
    }
    let mut count = 0;
    while dates.contains(&fmt(day)) {
        count += 1;
        day -= Duration::days(1);
    }
    count
}

/// 습관 통계(달성률·최장 연속·요일 패턴).
pub struct Stats {
    pub total: usize,           // 총 완료 일수
    pub span: i64,              // 첫 완료일~오늘(일)
    pub longest: i64,           // 가장 긴 연속
    pub by_weekday: [usize; 7], // 월~일 완료 횟수
}

impl Stats {
    /// '가장 꾸준한 요일' — **유일한 1등**일 때만 `Some((요일 index 0=월, 횟수))`.
    /// 동률(여러 요일이 똑같이 최다)이면 패턴이 뚜렷하지 않으니 None(단정하지 않음).
    pub fn dominant_weekday(&self) -> Option<(usize, usize)> {
        let max = *self.by_weekday.iter().max()?;
        if max == 0 {
            return None;
        }
        let leaders: Vec<usize> = (0..7).filter(|&i| self.by_weekday[i] == max).collect();
        match leaders.as_slice() {
            [only] => Some((*only, max)),
            _ => None, // 동률 → 생략
        }
    }
}

/// 완료 날짜 집합으로 통계를 낸다. 완료 기록이 없으면 None.
pub fn stats(dates: &HashSet<String>, today: NaiveDate) -> Option<Stats> {
    let mut days: Vec<NaiveDate> = dates
        .iter()
        .filter_map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .collect();
    days.sort();
    let first = *days.first()?;
    let span = (today - first).num_days() + 1;
    // 가장 긴 연속.
    let (mut longest, mut run) = (1i64, 1i64);
    for w in days.windows(2) {
        match (w[1] - w[0]).num_days() {
            1 => {
                run += 1;
                longest = longest.max(run);
            }
            0 => {}
            _ => run = 1,
        }
    }
    // 요일별(월=0 … 일=6).
    let mut by_weekday = [0usize; 7];
    for d in &days {
        by_weekday[d.weekday().num_days_from_monday() as usize] += 1;
    }
    Some(Stats {
        total: days.len(),
        span,
        longest,
        by_weekday,
    })
}

impl Habit {
    pub fn date_set(&self) -> HashSet<String> {
        self.dates.iter().cloned().collect()
    }
    pub fn done_today(&self, today_str: &str) -> bool {
        self.dates.iter().any(|d| d == today_str)
    }
    pub fn streak(&self, today: NaiveDate) -> i64 {
        streak(&self.date_set(), today)
    }
}

impl HabitStore {
    pub fn load() -> Result<Self> {
        let path = store_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(crate::util::load_json_or_recover(&path))
    }

    pub fn save(&self) -> Result<()> {
        crate::util::atomic_write(
            &store_path()?,
            serde_json::to_string_pretty(self)?.as_bytes(),
        )?;
        Ok(())
    }

    pub fn add(&mut self, name: &str) -> Result<u64> {
        let id = self
            .items
            .iter()
            .map(|x| x.id)
            .max()
            .unwrap_or(0)
            .max(self.next_id)
            .saturating_add(1);
        self.next_id = id;
        self.items.push(Habit {
            id,
            name: name.trim().to_string(),
            dates: Vec::new(),
        });
        self.save()?;
        Ok(id)
    }

    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.items.len();
        self.items.retain(|h| h.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// 이름 또는 id 문자열로 습관을 찾아 오늘 완료 표시(idempotent).
    pub fn check(&mut self, key: &str) -> Result<Option<(String, i64)>> {
        let today_s = today_str();
        let by_id: Option<u64> = key.parse().ok();
        let found = self
            .items
            .iter_mut()
            .find(|h| Some(h.id) == by_id || h.name == key);
        if let Some(h) = found {
            if !h.done_today(&today_s) {
                h.dates.push(today_s.clone());
            }
            let result = (h.name.clone(), h.streak(today()));
            self.save()?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

/// 연속 일수가 기념할 만한 고비면 축하용 라벨을 돌려준다(자랑 카드 공유 유도 트리거).
/// 흔치 않은 마일스톤에서만 떠 스팸이 되지 않는다.
pub fn milestone(streak: i64) -> Option<&'static str> {
    match streak {
        7 => Some("일주일 연속 🎉"),
        14 => Some("2주 연속 🎉"),
        30 => Some("한 달 연속 🎊"),
        50 => Some("50일 연속 🎊"),
        100 => Some("백일 연속 🏆"),
        200 => Some("200일 연속 🏆"),
        365 => Some("1년 연속 👑"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(days: &[&str]) -> HashSet<String> {
        days.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn milestone_only_at_notable_streaks() {
        assert_eq!(milestone(7), Some("일주일 연속 🎉"));
        assert_eq!(milestone(30), Some("한 달 연속 🎊"));
        assert_eq!(milestone(100), Some("백일 연속 🏆"));
        assert_eq!(milestone(365), Some("1년 연속 👑"));
        // 평범한 날엔 안 뜬다(매일 알림 스팸 방지).
        assert_eq!(milestone(1), None);
        assert_eq!(milestone(8), None);
        assert_eq!(milestone(31), None);
        assert_eq!(milestone(99), None);
    }

    #[test]
    fn streak_counts_consecutive_ending_today() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 5).unwrap();
        let s = set(&["2026-06-03", "2026-06-04", "2026-06-05"]);
        assert_eq!(streak(&s, today), 3);
    }

    #[test]
    fn stats_computes_rate_longest_and_weekday() {
        // 06-01(월)·02(화)·03(수)·05(금) 완료, 오늘 06-06(토).
        let s = set(&["2026-06-01", "2026-06-02", "2026-06-03", "2026-06-05"]);
        let st = stats(&s, NaiveDate::from_ymd_opt(2026, 6, 6).unwrap()).unwrap();
        assert_eq!(st.total, 4);
        assert_eq!(st.span, 6); // 06-01~06-06
        assert_eq!(st.longest, 3); // 01·02·03
        assert_eq!(st.by_weekday[0], 1); // 월
        assert_eq!(st.by_weekday[2], 1); // 수
        assert_eq!(st.by_weekday[4], 1); // 금
        assert_eq!(st.by_weekday[5], 0); // 토
                                         // 완료 없으면 None.
        assert!(stats(&set(&[]), NaiveDate::from_ymd_opt(2026, 6, 6).unwrap()).is_none());
    }

    #[test]
    fn dominant_weekday_only_on_unique_winner() {
        // 월이 유일한 1등(2번) → Some(월).
        let s = set(&["2026-06-01", "2026-06-08", "2026-06-02"]); // 월·월·화
        let st = stats(&s, NaiveDate::from_ymd_opt(2026, 6, 9).unwrap()).unwrap();
        assert_eq!(st.dominant_weekday(), Some((0, 2)));
        // 월·화·수 각 1번(동률) → None(단정하지 않음).
        let s = set(&["2026-06-01", "2026-06-02", "2026-06-03"]);
        let st = stats(&s, NaiveDate::from_ymd_opt(2026, 6, 6).unwrap()).unwrap();
        assert_eq!(st.dominant_weekday(), None);
    }

    #[test]
    fn streak_alive_if_yesterday_done_but_not_today() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 5).unwrap();
        let s = set(&["2026-06-03", "2026-06-04"]); // 오늘(5일) 아직 안 함
        assert_eq!(streak(&s, today), 2); // 어제까지 이어짐
    }

    #[test]
    fn streak_zero_if_gap() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 5).unwrap();
        let s = set(&["2026-06-01", "2026-06-02"]); // 3일 끊김
        assert_eq!(streak(&s, today), 0);
    }

    #[test]
    fn streak_breaks_on_internal_gap() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 5).unwrap();
        let s = set(&["2026-06-02", "2026-06-04", "2026-06-05"]); // 3일 빠짐
        assert_eq!(streak(&s, today), 2); // 4,5일만
    }
}
