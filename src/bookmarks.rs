//! 즐겨찾기 + 빠른 열기 — 자주 가는 사이트/폴더/앱을 한국어 단축어로 연다.
//!
//! `열기 노션`처럼 단축어로 URL·경로·앱을 OS 기본 프로그램으로 실행한다.
//!
//! 저장 위치: `~/.local/share/wonjang/bookmarks.json`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: u64,
    pub name: String,
    /// 열 대상: URL, 파일/폴더 경로, 앱 이름 등.
    pub target: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BookmarkStore {
    #[serde(default)]
    pub items: Vec<Bookmark>,
    #[serde(default)]
    next_id: u64,
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("bookmarks.json"))
}

impl BookmarkStore {
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

    pub fn add(&mut self, name: &str, target: &str) -> Result<u64> {
        let name = name.trim();
        if name.is_empty() || target.trim().is_empty() {
            bail!("이름과 대상이 모두 필요합니다");
        }
        let id = self
            .items
            .iter()
            .map(|x| x.id)
            .max()
            .unwrap_or(0)
            .max(self.next_id)
            + 1;
        self.next_id = id;
        self.items.push(Bookmark {
            id,
            name: name.to_string(),
            target: target.trim().to_string(),
        });
        self.save()?;
        Ok(id)
    }

    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.items.len();
        self.items.retain(|b| b.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// 이름(또는 id)으로 즐겨찾기를 찾는다.
    pub fn find(&self, key: &str) -> Option<&Bookmark> {
        let by_id: Option<u64> = key.parse().ok();
        self.items
            .iter()
            .find(|b| b.name == key || Some(b.id) == by_id)
    }
}

/// 대상(URL/경로/앱)을 OS 기본 프로그램으로 연다.
pub fn open_target(target: &str) -> Result<()> {
    let result = match std::env::consts::OS {
        "macos" => Command::new("open")
            .arg(target)
            .stderr(Stdio::null())
            .spawn(),
        "linux" => Command::new("xdg-open")
            .arg(target)
            .stderr(Stdio::null())
            .spawn(),
        "windows" => Command::new("cmd")
            .args(["/C", "start", "", target])
            .spawn(),
        other => bail!("이 OS({other})에서는 열기를 지원하지 않습니다"),
    };
    result.with_context(|| format!("'{target}' 열기에 실패했습니다"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_find() {
        let mut s = BookmarkStore::default();
        // save를 피하려 직접 구성.
        s.items.push(Bookmark {
            id: 1,
            name: "노션".into(),
            target: "https://notion.so".into(),
        });
        s.items.push(Bookmark {
            id: 2,
            name: "다운로드".into(),
            target: "~/Downloads".into(),
        });
        assert_eq!(s.find("노션").unwrap().target, "https://notion.so");
        assert_eq!(s.find("2").unwrap().name, "다운로드"); // id로도
        assert!(s.find("없음").is_none());
    }
}
