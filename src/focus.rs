//! 집중(뽀모도로) 트래커 — 집중 세션을 기록하고 하루 집중 시간을 누적한다.
//!
//! 공부·작업하는 사용자를 위한 기능. `집중 25 코딩`처럼 시작하면 끝나는 시각에
//! 알림이 울리도록 약속을 등록(스케줄러가 켜져 있으면)하고, 세션을 기록해 오늘
//! 총 집중 시간을 보여준다.
//!
//! 저장 위치: `~/.local/share/wonjang/focus.json`

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSession {
    pub id: u64,
    /// 날짜(YYYY-MM-DD).
    pub date: String,
    pub minutes: i64,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FocusStore {
    #[serde(default)]
    pub items: Vec<FocusSession>,
    #[serde(default)]
    next_id: u64,
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("focus.json"))
}

pub fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 분을 "N시간 M분" 또는 "M분"으로.
pub fn fmt_minutes(m: i64) -> String {
    if m >= 60 {
        let h = m / 60;
        let r = m % 60;
        if r == 0 {
            format!("{h}시간")
        } else {
            format!("{h}시간 {r}분")
        }
    } else {
        format!("{m}분")
    }
}

impl FocusStore {
    pub fn load() -> Result<Self> {
        let path = store_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&text).unwrap_or_default())
    }

    pub fn save(&self) -> Result<()> {
        crate::util::atomic_write(
            &store_path()?,
            serde_json::to_string_pretty(self)?.as_bytes(),
        )?;
        Ok(())
    }

    /// 오늘 날짜로 집중 세션을 기록한다.
    pub fn add(&mut self, minutes: i64, label: &str) -> Result<u64> {
        let id = self
            .items
            .iter()
            .map(|x| x.id)
            .max()
            .unwrap_or(0)
            .max(self.next_id)
            .saturating_add(1);
        self.next_id = id;
        self.items.push(FocusSession {
            id,
            date: today_str(),
            minutes,
            label: label.trim().to_string(),
        });
        self.save()?;
        Ok(id)
    }

    pub fn today_total(&self, date: &str) -> i64 {
        self.items
            .iter()
            .filter(|s| s.date == date)
            .map(|s| s.minutes)
            .sum()
    }

    pub fn today_count(&self, date: &str) -> usize {
        self.items.iter().filter(|s| s.date == date).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_and_count() {
        let mut s = FocusStore::default();
        s.items.push(FocusSession {
            id: 1,
            date: "2026-06-01".into(),
            minutes: 25,
            label: "코딩".into(),
        });
        s.items.push(FocusSession {
            id: 2,
            date: "2026-06-01".into(),
            minutes: 50,
            label: "독서".into(),
        });
        s.items.push(FocusSession {
            id: 3,
            date: "2026-05-31".into(),
            minutes: 25,
            label: "".into(),
        });
        assert_eq!(s.today_total("2026-06-01"), 75);
        assert_eq!(s.today_count("2026-06-01"), 2);
    }

    #[test]
    fn fmt_minutes_cases() {
        assert_eq!(fmt_minutes(25), "25분");
        assert_eq!(fmt_minutes(60), "1시간");
        assert_eq!(fmt_minutes(95), "1시간 35분");
    }
}
