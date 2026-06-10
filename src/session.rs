//! 세션 영속화.
//!
//! 대화 기록(메시지 전체)을 디스크에 저장해, 터미널을 닫았다가 다시 열어도
//! `--continue`로 이전 대화를 이어갈 수 있게 한다.
//!
//! 저장 위치: `~/.local/share/wonjang/sessions/session-<ms>.json`

use crate::llm::Message;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
struct SessionFile {
    created_ms: u128,
    messages: Vec<Message>,
}

pub struct Session {
    path: PathBuf,
    created_ms: u128,
}

fn sessions_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang")
        .join("sessions");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir)
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

impl Session {
    /// 새 세션을 만든다.
    pub fn new() -> Result<Self> {
        let created_ms = now_ms();
        let path = sessions_dir()?.join(format!("session-{created_ms}.json"));
        Ok(Self { path, created_ms })
    }

    /// 가장 최근 세션을 이어간다(없으면 새 세션).
    pub fn latest_or_new() -> Result<(Self, Vec<Message>)> {
        match latest_path()? {
            Some(path) => {
                let (created_ms, messages) = read_file(&path)?;
                Ok((Self { path, created_ms }, messages))
            }
            None => Ok((Self::new()?, Vec::new())),
        }
    }

    /// 현재 메시지 전체를 저장한다.
    pub fn save(&self, messages: &[Message]) -> Result<()> {
        let file = SessionFile {
            created_ms: self.created_ms,
            messages: messages.to_vec(),
        };
        let json = serde_json::to_string_pretty(&file)?;
        crate::util::atomic_write(&self.path, json.as_bytes())
            .with_context(|| format!("세션을 저장할 수 없습니다: {}", self.path.display()))?;
        Ok(())
    }
}

/// 가장 최근 세션 파일 경로.
fn latest_path() -> Result<Option<PathBuf>> {
    let dir = sessions_dir()?;
    let mut newest: Option<(u128, PathBuf)> = None;
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if let Some(ms) = parse_ms(&path) {
            if newest.as_ref().map(|(m, _)| ms > *m).unwrap_or(true) {
                newest = Some((ms, path));
            }
        }
    }
    Ok(newest.map(|(_, p)| p))
}

/// 파일명에서 타임스탬프(ms)를 파싱.
fn parse_ms(path: &Path) -> Option<u128> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_prefix("session-")?.strip_suffix(".json")?;
    stem.parse().ok()
}

fn read_file(path: &Path) -> Result<(u128, Vec<Message>)> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("세션을 읽을 수 없습니다: {}", path.display()))?;
    let file: SessionFile =
        serde_json::from_str(&text).context("세션 파일(JSON) 형식이 올바르지 않습니다")?;
    Ok((file.created_ms, file.messages))
}

/// 세션 통계: (세션 수, 가장 이른 세션 타임스탬프 ms). 파일명만 읽어 가볍다.
pub fn stats() -> Result<(usize, Option<u128>)> {
    let dir = sessions_dir()?;
    let mut count = 0usize;
    let mut earliest: Option<u128> = None;
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if let Some(ms) = parse_ms(&path) {
            count += 1;
            if earliest.map(|e| ms < e).unwrap_or(true) {
                earliest = Some(ms);
            }
        }
    }
    Ok((count, earliest))
}

/// 저장된 세션 목록(최신순)을 (경로, 미리보기, 메시지 수)로 반환.
pub fn list() -> Result<Vec<(PathBuf, String, usize)>> {
    let dir = sessions_dir()?;
    let mut items: Vec<(u128, PathBuf, String, usize)> = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if let Some(ms) = parse_ms(&path) {
            if let Ok((_, messages)) = read_file(&path) {
                let preview = messages
                    .iter()
                    .find(|m| m.role == "user")
                    .and_then(|m| m.content.clone())
                    .unwrap_or_else(|| "(빈 세션)".to_string());
                let preview = preview
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(60)
                    .collect();
                items.push((ms, path, preview, messages.len()));
            }
        }
    }
    items.sort_by_key(|it| std::cmp::Reverse(it.0)); // 최신순
    Ok(items.into_iter().map(|(_, p, pv, n)| (p, pv, n)).collect())
}
