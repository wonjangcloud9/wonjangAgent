//! 습관 트래커 — 매일 습관을 체크하고 연속 일수(streak)를 챙긴다.
//!
//! "갓생" 문화에 맞춘 자기계발 기능. 습관별로 완료한 날짜를 모으고, 오늘까지
//! 이어진 연속 일수를 계산해 동기를 준다. 브리핑에도 함께 보여준다.
//!
//! 저장 위치: `~/.local/share/wonjang/habits.json`

use anyhow::{Context, Result};
use chrono::{Duration, Local, NaiveDate};
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
        let text = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&text).unwrap_or_default())
    }

    pub fn save(&self) -> Result<()> {
        std::fs::write(store_path()?, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn add(&mut self, name: &str) -> Result<u64> {
        self.next_id += 1;
        let id = self.next_id;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn set(days: &[&str]) -> HashSet<String> {
        days.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn streak_counts_consecutive_ending_today() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 5).unwrap();
        let s = set(&["2026-06-03", "2026-06-04", "2026-06-05"]);
        assert_eq!(streak(&s, today), 3);
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
