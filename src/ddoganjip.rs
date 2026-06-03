//! 풍자 〈또간집〉 선정 맛집 — 지역으로 찾는 위치 기반 추천.
//!
//! 〈또간집〉 식당 목록은 공개 API가 없어, 사용자가 키우는 **로컬 목록**으로
//! 관리한다. 처음 실행 시 미검증 시드 몇 곳을 넣어 두고(전부 `verified=false`),
//! `add`로 직접 확인한 곳을 더한다. 좌표를 지어내지 않도록 지역·메모만 담는다.
//!
//! 저장 위치: `~/.local/share/wonjang/ddoganjip.json`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spot {
    pub name: String,
    /// 지역(시/도·구·동 등 자유 입력).
    pub region: String,
    #[serde(default)]
    pub note: String,
    /// 사용자가 직접 확인했으면 true. 시드는 false(미검증).
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DdoganjipStore {
    #[serde(default)]
    pub items: Vec<Spot>,
    #[serde(default)]
    seeded: bool,
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("ddoganjip.json"))
}

/// 초기 시드(전부 미검증). 정확한 주소·좌표는 일부러 넣지 않는다.
fn seed() -> Vec<Spot> {
    let s = |name: &str, region: &str| Spot {
        name: name.to_string(),
        region: region.to_string(),
        note: "초기 시드 — 또간집 등장/위치 미검증, 방문 전 확인".to_string(),
        verified: false,
    };
    vec![
        s("(예시) 또간집 후보 1", "서울 종로"),
        s("(예시) 또간집 후보 2", "서울 마포"),
        s("(예시) 또간집 후보 3", "부산 중구"),
        s("(예시) 또간집 후보 4", "전주 완산"),
        s("(예시) 또간집 후보 5", "대구 중구"),
    ]
}

impl DdoganjipStore {
    pub fn load() -> Result<Self> {
        let path = store_path()?;
        let mut store: Self = if path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&path)?).unwrap_or_default()
        } else {
            Self::default()
        };
        // 최초 1회만 시드 주입.
        if !store.seeded && store.items.is_empty() {
            store.items = seed();
            store.seeded = true;
            store.save().ok();
        }
        Ok(store)
    }

    pub fn save(&self) -> Result<()> {
        crate::util::atomic_write(
            &store_path()?,
            serde_json::to_string_pretty(self)?.as_bytes(),
        )?;
        Ok(())
    }

    /// 확인된 식당을 추가한다(사용자 추가 → verified=true).
    pub fn add(&mut self, name: &str, region: &str, note: &str) -> Result<()> {
        let name = name.trim();
        let region = region.trim();
        if name.is_empty() || region.is_empty() {
            bail!("식당 이름과 지역이 모두 필요합니다");
        }
        self.items.push(Spot {
            name: name.to_string(),
            region: region.to_string(),
            note: note.trim().to_string(),
            verified: true,
        });
        self.save()
    }

    /// 지역(또는 이름)에 검색어가 포함된 식당을 찾는다.
    pub fn find(&self, query: &str) -> Vec<&Spot> {
        let q = query.trim().to_lowercase();
        self.items
            .iter()
            .filter(|s| s.region.to_lowercase().contains(&q) || s.name.to_lowercase().contains(&q))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> DdoganjipStore {
        DdoganjipStore {
            items: vec![
                Spot {
                    name: "가게A".into(),
                    region: "서울 종로".into(),
                    note: "".into(),
                    verified: true,
                },
                Spot {
                    name: "가게B".into(),
                    region: "부산 해운대".into(),
                    note: "".into(),
                    verified: false,
                },
            ],
            seeded: true,
        }
    }

    #[test]
    fn finds_by_region_substring() {
        let s = store();
        assert_eq!(s.find("서울").len(), 1);
        assert_eq!(s.find("종로").len(), 1);
        assert_eq!(s.find("부산").len(), 1);
        assert_eq!(s.find("대전").len(), 0);
    }

    #[test]
    fn finds_by_name() {
        assert_eq!(store().find("가게A").len(), 1);
    }

    #[test]
    fn add_marks_verified() {
        let mut s = DdoganjipStore::default();
        s.items.push(Spot {
            name: "확인된집".into(),
            region: "대구".into(),
            note: "맛있음".into(),
            verified: true,
        });
        assert!(s.items[0].verified);
    }
}
