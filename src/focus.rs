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

/// 한 번의 집중 세션 상한(분) — 길어야 하루. 그 이상은 오타로 본다.
/// (상한이 없으면 `집중 200000000000000000`처럼 말도 안 되는 값이 누적돼 카드가 깨진다.)
pub const MAX_SESSION_MIN: i64 = 24 * 60;

/// 집중 시간(분) 유효성 검사. 통과하면 Ok, 아니면 사용자용 한국어 메시지.
pub fn check_minutes(m: i64) -> Result<(), String> {
    if m <= 0 {
        return Err("집중 시간은 1분 이상이어야 합니다. 예: wonjang 집중 25 코딩".to_string());
    }
    if m > MAX_SESSION_MIN {
        return Err(
            "집중 시간이 너무 길어요(최대 24시간=1440분). 예: wonjang 집중 25 코딩".to_string(),
        );
    }
    Ok(())
}

/// 합산 시 비정상(0 이하·24시간 초과) 항목은 0으로 무시한다 — 옛/수기편집 오염 데이터가
/// 카드·현황 합계를 깨뜨리는 것을 표시 단계에서도 방어한다(입력 검증과 별개의 방어선).
fn sane_minutes(m: i64) -> i64 {
    if m > 0 && m <= MAX_SESSION_MIN {
        m
    } else {
        0
    }
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
        Ok(crate::util::load_json_or_recover(&path))
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
            .map(|s| sane_minutes(s.minutes))
            .sum()
    }

    pub fn today_count(&self, date: &str) -> usize {
        self.items
            .iter()
            .filter(|s| s.date == date && sane_minutes(s.minutes) > 0)
            .count()
    }

    /// from(YYYY-MM-DD) 이후(포함) 집중 합계 — 최근 N일 추세용. YYYY-MM-DD는
    /// 사전순 비교가 곧 날짜순이라 문자열 `>=`로 안전하게 판정.
    pub fn since_total(&self, from: &str) -> i64 {
        self.items
            .iter()
            .filter(|s| s.date.as_str() >= from)
            .map(|s| sane_minutes(s.minutes))
            .sum()
    }

    /// 특정 월(YYYY-MM) 집중 합계.
    pub fn month_total(&self, ym: &str) -> i64 {
        self.items
            .iter()
            .filter(|s| s.date.starts_with(ym))
            .map(|s| sane_minutes(s.minutes))
            .sum()
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
    fn garbage_entry_ignored_in_totals() {
        // 옛/수기편집 오염값(2e17분)이 합계·횟수를 깨뜨리지 않아야(표시 단계 방어).
        let mut s = FocusStore::default();
        s.items.push(FocusSession {
            id: 1,
            date: "2026-06-04".into(),
            minutes: 200_000_000_000_000_000,
            label: "코딩".into(),
        });
        s.items.push(FocusSession {
            id: 2,
            date: "2026-06-04".into(),
            minutes: 25,
            label: "코딩".into(),
        });
        assert_eq!(s.today_total("2026-06-04"), 25); // garbage 제외
        assert_eq!(s.today_count("2026-06-04"), 1); // garbage는 세션으로 안 셈
        assert_eq!(s.month_total("2026-06"), 25);
        assert_eq!(s.since_total("2026-06-01"), 25);
    }

    #[test]
    fn since_and_month_totals() {
        let mut s = FocusStore::default();
        for (d, m) in [("2026-05-31", 25), ("2026-06-01", 50), ("2026-06-03", 30)] {
            s.items.push(FocusSession {
                id: 0,
                date: d.into(),
                minutes: m,
                label: "".into(),
            });
        }
        // 6/01 이후(포함) = 50 + 30 = 80, 5/31은 제외.
        assert_eq!(s.since_total("2026-06-01"), 80);
        // 이번 달(6월) = 80, 5월 1건은 제외.
        assert_eq!(s.month_total("2026-06"), 80);
        assert_eq!(s.month_total("2026-05"), 25);
    }

    #[test]
    fn fmt_minutes_cases() {
        assert_eq!(fmt_minutes(25), "25분");
        assert_eq!(fmt_minutes(60), "1시간");
        assert_eq!(fmt_minutes(95), "1시간 35분");
    }

    #[test]
    fn check_minutes_rejects_absurd_and_nonpositive() {
        assert!(check_minutes(25).is_ok());
        assert!(check_minutes(1).is_ok());
        assert!(check_minutes(MAX_SESSION_MIN).is_ok()); // 24시간 경계는 허용
        assert!(check_minutes(0).is_err());
        assert!(check_minutes(-5).is_err());
        assert!(check_minutes(MAX_SESSION_MIN + 1).is_err());
        // 카드를 깨뜨렸던 실제 오염 값.
        assert!(check_minutes(200_000_000_000_000_000).is_err());
    }
}
