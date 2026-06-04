//! 시세 알림(가격 감시) — "비트코인 1억 넘으면 알려줘".
//!
//! 목표가를 등록하면 스케줄러 데몬이 업비트 시세를 지켜보다가 목표가에 도달하면
//! 설정된 채널(카카오/디스코드/텔레그램)로 푸시한다. 24시간 비서가 대신 지켜본다.
//!
//! 저장 위치: `~/.local/share/wonjang/watches.json`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watch {
    pub id: u64,
    pub symbol: String,
    pub market: String,
    pub target: f64,
    /// true이면 목표가 '이상'에서, false이면 '이하'에서 알림.
    pub above: bool,
    /// 감시 종류: "coin"(업비트) 또는 "fx"(환율).
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub triggered: bool,
}

fn default_kind() -> String {
    "coin".to_string()
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WatchStore {
    #[serde(default)]
    pub items: Vec<Watch>,
    #[serde(default)]
    next_id: u64,
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("watches.json"))
}

impl WatchStore {
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

    /// 알림을 추가한다. `above`는 현재가 기준으로, `kind`는 "coin"/"fx".
    pub fn add(&mut self, symbol: &str, target: f64, above: bool, kind: &str) -> Result<u64> {
        let sym = symbol.trim().to_uppercase();
        let id = self
            .items
            .iter()
            .map(|x| x.id)
            .max()
            .unwrap_or(0)
            .max(self.next_id)
            .saturating_add(1);
        self.next_id = id;
        self.items.push(Watch {
            id,
            market: format!("KRW-{sym}"),
            symbol: sym,
            target,
            above,
            kind: kind.to_string(),
            triggered: false,
        });
        self.save()?;
        Ok(id)
    }

    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.items.len();
        self.items.retain(|w| w.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    pub fn active(&self) -> Vec<&Watch> {
        self.items.iter().filter(|w| !w.triggered).collect()
    }

    pub fn mark_triggered(&mut self, id: u64) {
        if let Some(w) = self.items.iter_mut().find(|w| w.id == id) {
            w.triggered = true;
        }
    }
}

/// 현재가가 알림 조건을 만족하는가(이미 발동된 건 제외).
pub fn should_trigger(w: &Watch, price: f64) -> bool {
    if w.triggered {
        return false;
    }
    if w.above {
        price >= w.target
    } else {
        price <= w.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(target: f64, above: bool) -> Watch {
        Watch {
            id: 1,
            symbol: "BTC".into(),
            market: "KRW-BTC".into(),
            target,
            above,
            kind: "coin".into(),
            triggered: false,
        }
    }

    #[test]
    fn above_triggers_when_reached() {
        let watch = w(110_000_000.0, true);
        assert!(!should_trigger(&watch, 109_000_000.0));
        assert!(should_trigger(&watch, 110_000_000.0));
        assert!(should_trigger(&watch, 111_000_000.0));
    }

    #[test]
    fn below_triggers_when_dropped() {
        let watch = w(100_000_000.0, false);
        assert!(!should_trigger(&watch, 101_000_000.0));
        assert!(should_trigger(&watch, 100_000_000.0));
        assert!(should_trigger(&watch, 99_000_000.0));
    }

    #[test]
    fn triggered_does_not_refire() {
        let mut watch = w(100.0, true);
        watch.triggered = true;
        assert!(!should_trigger(&watch, 200.0));
    }
}
