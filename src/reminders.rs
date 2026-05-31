//! 약속·알림(reminder) — 24시간 비서의 핵심.
//!
//! 사용자의 약속/할 일을 시각과 함께 저장하고, 크론 데몬이 때가 되면 데스크탑
//! 알림으로 띄운다. 시각은 epoch(초)로 저장해 타임존 의존을 피하며, 사람 시각→
//! epoch 변환은 에이전트가 `date` 등으로 처리한다.
//!
//! 저장 위치: `~/.local/share/wonjang/reminders.json`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: u64,
    /// 알릴 시각(epoch 초).
    pub at_unix: i64,
    pub title: String,
    #[serde(default)]
    pub notified: bool,
    /// 반복 주기(초). 있으면 알린 뒤 다음 회차로 재예약된다.
    #[serde(default)]
    pub repeat_secs: Option<i64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReminderStore {
    #[serde(default)]
    pub items: Vec<Reminder>,
    #[serde(default)]
    next_id: u64,
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("reminders.json"))
}

impl ReminderStore {
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

    /// 새 알림을 추가하고 id를 반환한다(`repeat_secs`가 있으면 반복).
    pub fn add(&mut self, at_unix: i64, title: &str, repeat_secs: Option<i64>) -> Result<u64> {
        self.next_id += 1;
        let id = self.next_id;
        self.items.push(Reminder {
            id,
            at_unix,
            title: title.trim().to_string(),
            notified: false,
            repeat_secs: repeat_secs.filter(|s| *s > 0),
        });
        self.items.sort_by_key(|r| r.at_unix);
        self.save()?;
        Ok(id)
    }

    /// 알림이 발화된 뒤 처리: 반복이면 다음 회차로 재예약, 아니면 완료 표시.
    pub fn handle_fired(&mut self, id: u64, now: i64) {
        if let Some(r) = self.items.iter_mut().find(|r| r.id == id) {
            match r.repeat_secs {
                Some(p) if p > 0 => {
                    let mut next = r.at_unix;
                    while next <= now {
                        next += p;
                    }
                    r.at_unix = next;
                    r.notified = false;
                }
                _ => r.notified = true,
            }
        }
        self.items.sort_by_key(|r| r.at_unix);
    }

    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.items.len();
        self.items.retain(|r| r.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// 아직 알리지 않은, 시각이 지난 알림들.
    pub fn due(&self, now: i64) -> Vec<Reminder> {
        self.items
            .iter()
            .filter(|r| !r.notified && r.at_unix <= now)
            .cloned()
            .collect()
    }

    /// 아직 안 지난(예정된) 알림들(시각순).
    pub fn upcoming(&self, now: i64) -> Vec<&Reminder> {
        let mut v: Vec<&Reminder> = self.items.iter().filter(|r| r.at_unix > now).collect();
        v.sort_by_key(|r| r.at_unix);
        v
    }
}

/// epoch 시각을 지금 기준 상대 표현으로(타임존 비의존).
pub fn relative(at_unix: i64, now: i64) -> String {
    let diff = at_unix - now;
    if diff <= 0 {
        return "지남".to_string();
    }
    let m = diff / 60;
    if m < 60 {
        format!("{m}분 후")
    } else if m < 60 * 24 {
        format!("{}시간 {}분 후", m / 60, m % 60)
    } else {
        format!("{}일 후", m / (60 * 24))
    }
}

/// 반복 주기를 사람이 읽는 라벨로(없으면 빈 문자열).
pub fn repeat_label(repeat_secs: Option<i64>) -> String {
    match repeat_secs {
        Some(86400) => " · 매일 반복".to_string(),
        Some(604800) => " · 매주 반복".to_string(),
        Some(3600) => " · 매시간 반복".to_string(),
        Some(s) if s % 86400 == 0 => format!(" · {}일마다 반복", s / 86400),
        Some(s) if s % 3600 == 0 => format!(" · {}시간마다 반복", s / 3600),
        Some(s) => format!(" · {}분마다 반복", s / 60),
        None => String::new(),
    }
}

/// 데스크탑 알림(베스트 에포트, OS별).
pub fn desktop_notify(title: &str, body: &str) {
    let t = title.replace('"', "'");
    let b = body.replace('"', "'");
    let _ = match std::env::consts::OS {
        "macos" => std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!("display notification \"{b}\" with title \"{t}\""))
            .spawn(),
        "linux" => std::process::Command::new("notify-send")
            .arg(&t)
            .arg(&b)
            .spawn(),
        _ => return,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_and_notified_logic() {
        let mut s = ReminderStore::default();
        // save를 호출하지 않도록 직접 구성.
        s.items.push(Reminder {
            id: 1,
            at_unix: 1000,
            title: "치과".into(),
            notified: false,
            repeat_secs: None,
        });
        s.items.push(Reminder {
            id: 2,
            at_unix: 5000,
            title: "회의".into(),
            notified: false,
            repeat_secs: None,
        });
        assert_eq!(s.due(2000).len(), 1); // id1만 지남
        assert_eq!(s.due(2000)[0].title, "치과");
        s.handle_fired(1, 2000); // 반복 아님 → 완료 표시
        assert_eq!(s.due(2000).len(), 0); // 알림 처리됨
        assert_eq!(s.upcoming(2000).len(), 1); // id2 예정
    }

    #[test]
    fn recurring_reschedules_to_future() {
        let mut s = ReminderStore::default();
        s.items.push(Reminder {
            id: 1,
            at_unix: 1000,
            title: "약 먹기".into(),
            notified: false,
            repeat_secs: Some(86400), // 매일
        });
        // 1000에 발화, 지금이 2000이면 다음 회차는 1000+86400.
        s.handle_fired(1, 2000);
        let r = &s.items[0];
        assert_eq!(r.at_unix, 1000 + 86400);
        assert!(!r.notified); // 반복이므로 완료 표시 안 함
    }

    #[test]
    fn repeat_label_format() {
        assert_eq!(repeat_label(Some(86400)), " · 매일 반복");
        assert_eq!(repeat_label(Some(3600)), " · 매시간 반복");
        assert_eq!(repeat_label(None), "");
    }

    #[test]
    fn relative_format() {
        let now = 1_000_000;
        assert_eq!(relative(now - 10, now), "지남");
        assert_eq!(relative(now + 1800, now), "30분 후");
        assert_eq!(relative(now + 3600 + 600, now), "1시간 10분 후");
        assert_eq!(relative(now + 86400 * 2, now), "2일 후");
    }
}
