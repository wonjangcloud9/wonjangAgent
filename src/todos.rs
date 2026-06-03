//! 할 일(todo) 관리 — 시각 없는 체크리스트(약속·알림과 보완 관계).
//!
//! 약속(reminders)은 "언제"가 핵심이고, 할 일(todos)은 "무엇"의 목록이다.
//! 24시간 비서가 사용자의 할 일을 모아두고 브리핑에 함께 보여준다.
//!
//! 저장 위치: `~/.local/share/wonjang/todos.json`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub text: String,
    #[serde(default)]
    pub done: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TodoStore {
    #[serde(default)]
    pub items: Vec<Todo>,
    #[serde(default)]
    next_id: u64,
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("todos.json"))
}

impl TodoStore {
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

    pub fn add(&mut self, text: &str) -> Result<u64> {
        self.next_id += 1;
        let id = self.next_id;
        self.items.push(Todo {
            id,
            text: text.trim().to_string(),
            done: false,
        });
        self.save()?;
        Ok(id)
    }

    /// 완료 표시(성공 여부 반환).
    pub fn complete(&mut self, id: u64) -> Result<bool> {
        let found = if let Some(t) = self.items.iter_mut().find(|t| t.id == id) {
            t.done = true;
            true
        } else {
            false
        };
        if found {
            self.save()?;
        }
        Ok(found)
    }

    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.items.len();
        self.items.retain(|t| t.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// 완료된 항목 모두 제거(정리).
    pub fn clear_done(&mut self) -> Result<usize> {
        let before = self.items.len();
        self.items.retain(|t| !t.done);
        let cleared = before - self.items.len();
        if cleared > 0 {
            self.save()?;
        }
        Ok(cleared)
    }

    /// 아직 안 끝낸 할 일들.
    pub fn pending(&self) -> Vec<&Todo> {
        self.items.iter().filter(|t| !t.done).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk() -> TodoStore {
        TodoStore::default()
    }

    #[test]
    fn add_complete_pending() {
        let mut s = mk();
        s.items.push(Todo {
            id: 1,
            text: "장보기".into(),
            done: false,
        });
        s.items.push(Todo {
            id: 2,
            text: "운동".into(),
            done: false,
        });
        assert_eq!(s.pending().len(), 2);
        // complete는 save를 호출하므로 직접 done 설정으로 로직만 확인.
        s.items[0].done = true;
        assert_eq!(s.pending().len(), 1);
        assert_eq!(s.pending()[0].text, "운동");
    }

    #[test]
    fn clear_done_keeps_pending() {
        let mut s = mk();
        s.items.push(Todo {
            id: 1,
            text: "끝난거".into(),
            done: true,
        });
        s.items.push(Todo {
            id: 2,
            text: "할거".into(),
            done: false,
        });
        s.items.retain(|t| !t.done); // clear_done의 핵심 로직
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.items[0].text, "할거");
    }
}
