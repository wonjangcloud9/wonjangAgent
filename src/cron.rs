//! 크론 스케줄러 — 무인 자동화.
//!
//! 사용자가 등록한 작업을 일정 간격으로 자동 실행한다. 타임존/요일 의존을
//! 피하기 위해 v0.6은 **간격 기반** 스케줄만 지원한다(의존성 없이 견고):
//!
//! - `@every 30m`, `@every 2h`, `@every 1d` 또는 단위만: `30m`, `2h`, `90s`
//! - `@minutely` / `@hourly` / `@daily` / `@weekly`
//!
//! 저장 위치: `~/.local/share/wonjang/cron.json`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronTask {
    pub id: u64,
    /// 원본 스케줄 문자열(예: "@every 30m").
    pub schedule: String,
    /// 실행할 요청(에이전트에게 전달).
    pub prompt: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 마지막 실행 시각(epoch ms).
    #[serde(default)]
    pub last_run_ms: Option<u128>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CronStore {
    #[serde(default)]
    pub tasks: Vec<CronTask>,
    #[serde(default)]
    next_id: u64,
}

pub fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("cron.json"))
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

impl CronStore {
    pub fn load() -> Result<Self> {
        let path = store_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&text).unwrap_or_default())
    }

    pub fn save(&self) -> Result<()> {
        let path = store_path()?;
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// 새 작업을 추가한다(스케줄 유효성 검증 포함).
    pub fn add(&mut self, schedule: &str, prompt: &str) -> Result<u64> {
        parse_schedule(schedule)?; // 검증
        self.next_id += 1;
        let id = self.next_id;
        self.tasks.push(CronTask {
            id,
            schedule: schedule.trim().to_string(),
            prompt: prompt.trim().to_string(),
            enabled: true,
            last_run_ms: None,
        });
        self.save()?;
        Ok(id)
    }

    /// id로 작업을 제거한다(제거 성공 여부 반환).
    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        let removed = self.tasks.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }
}

/// 간격 스케줄.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Schedule {
    pub interval: Duration,
}

/// 스케줄 문자열을 파싱한다.
pub fn parse_schedule(raw: &str) -> Result<Schedule> {
    let s = raw.trim();
    let interval = match s {
        "@minutely" => Duration::from_secs(60),
        "@hourly" => Duration::from_secs(3600),
        "@daily" => Duration::from_secs(86400),
        "@weekly" => Duration::from_secs(604800),
        _ => {
            let dur_str = s.strip_prefix("@every").map(str::trim).unwrap_or(s);
            parse_duration(dur_str)?
        }
    };
    if interval.as_secs() < 10 {
        bail!("간격이 너무 짧습니다(최소 10초). 예: '30m', '@every 2h', '@daily'");
    }
    Ok(Schedule { interval })
}

/// "30m", "2h", "90s", "1d" 형식을 Duration으로.
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        bail!("스케줄이 비어 있습니다. 예: '30m', '@every 2h', '@daily'");
    }
    let (num_part, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let n: u64 = num_part
        .parse()
        .with_context(|| format!("숫자를 해석할 수 없습니다: '{s}'"))?;
    let secs = match unit.trim() {
        "s" | "sec" | "초" => n,
        "m" | "min" | "분" => n * 60,
        "h" | "hr" | "시간" => n * 3600,
        "d" | "day" | "일" => n * 86400,
        other => bail!("알 수 없는 시간 단위: '{other}'. s/m/h/d 를 쓰세요"),
    };
    Ok(Duration::from_secs(secs))
}

/// 작업이 지금 실행 대상인지 판단.
pub fn is_due(task: &CronTask, now_ms: u128) -> bool {
    if !task.enabled {
        return false;
    }
    let sched = match parse_schedule(&task.schedule) {
        Ok(s) => s,
        Err(_) => return false,
    };
    match task.last_run_ms {
        None => true, // 한 번도 실행 안 함 → 즉시 실행.
        Some(last) => now_ms.saturating_sub(last) >= sched.interval.as_millis(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_named_schedules() {
        assert_eq!(
            parse_schedule("@hourly").unwrap().interval,
            Duration::from_secs(3600)
        );
        assert_eq!(
            parse_schedule("@daily").unwrap().interval,
            Duration::from_secs(86400)
        );
    }

    #[test]
    fn parse_duration_forms() {
        assert_eq!(
            parse_schedule("30m").unwrap().interval,
            Duration::from_secs(1800)
        );
        assert_eq!(
            parse_schedule("@every 2h").unwrap().interval,
            Duration::from_secs(7200)
        );
        assert_eq!(
            parse_schedule("1d").unwrap().interval,
            Duration::from_secs(86400)
        );
    }

    #[test]
    fn rejects_too_short_and_garbage() {
        assert!(parse_schedule("5s").is_err());
        assert!(parse_schedule("abc").is_err());
        assert!(parse_schedule("10x").is_err());
    }

    #[test]
    fn due_logic() {
        let mut t = CronTask {
            id: 1,
            schedule: "1h".into(),
            prompt: "x".into(),
            enabled: true,
            last_run_ms: None,
        };
        assert!(is_due(&t, 1_000_000)); // 미실행 → due
        t.last_run_ms = Some(1_000_000);
        assert!(!is_due(&t, 1_000_000 + 60_000)); // 1분 경과 → not due
        assert!(is_due(&t, 1_000_000 + 3_600_000)); // 1시간 경과 → due
        t.enabled = false;
        assert!(!is_due(&t, 1_000_000 + 7_200_000)); // 비활성 → not due
    }
}
