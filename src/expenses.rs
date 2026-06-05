//! 가계부(지출 관리) — 일상 지출을 기록하고 합계를 본다.
//!
//! "오늘 얼마 썼지", "이번달 식비"처럼 묻는 한국형 비서 기능. 금액은 원(KRW)
//! 정수로 저장하고, 날짜는 chrono(로컬)로 오늘을 기준한다.
//!
//! 저장 위치: `~/.local/share/wonjang/expenses.json`

use anyhow::{bail, Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expense {
    pub id: u64,
    pub amount: i64,
    pub category: String,
    #[serde(default)]
    pub note: String,
    /// 지출 날짜(YYYY-MM-DD).
    pub date: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExpenseStore {
    #[serde(default)]
    pub items: Vec<Expense>,
    #[serde(default)]
    next_id: u64,
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("expenses.json"))
}

/// 오늘 날짜(YYYY-MM-DD).
pub fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 이번 달(YYYY-MM).
pub fn this_month() -> String {
    Local::now().format("%Y-%m").to_string()
}

/// 금액을 천 단위 콤마 + '원'으로 포맷.
pub fn won(amount: i64) -> String {
    let neg = amount < 0;
    let digits = amount.abs().to_string();
    let mut out = String::new();
    let bytes = digits.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    format!("{}{}원", if neg { "-" } else { "" }, out)
}

impl ExpenseStore {
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

    /// 오늘 날짜로 지출을 추가한다.
    pub fn add(&mut self, amount: i64, category: &str, note: &str) -> Result<u64> {
        if amount <= 0 {
            bail!("금액은 1원 이상이어야 합니다");
        }
        let id = self
            .items
            .iter()
            .map(|x| x.id)
            .max()
            .unwrap_or(0)
            .max(self.next_id)
            .saturating_add(1);
        self.next_id = id;
        self.items.push(Expense {
            id,
            amount,
            category: category.trim().to_string(),
            note: note.trim().to_string(),
            date: today_str(),
        });
        self.save()?;
        Ok(id)
    }

    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.items.len();
        self.items.retain(|e| e.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// 특정 날짜(YYYY-MM-DD) 합계.
    pub fn total_on(&self, date: &str) -> i64 {
        self.items
            .iter()
            .filter(|e| e.date == date)
            .map(|e| e.amount)
            .sum()
    }

    /// 특정 월(YYYY-MM) 합계.
    pub fn total_in_month(&self, ym: &str) -> i64 {
        self.items
            .iter()
            .filter(|e| e.date.starts_with(ym))
            .map(|e| e.amount)
            .sum()
    }

    /// 특정 월의 분류별 합계(금액 내림차순).
    pub fn by_category_in_month(&self, ym: &str) -> Vec<(String, i64)> {
        let mut map: BTreeMap<String, i64> = BTreeMap::new();
        for e in self.items.iter().filter(|e| e.date.starts_with(ym)) {
            *map.entry(e.category.clone()).or_insert(0) += e.amount;
        }
        let mut v: Vec<(String, i64)> = map.into_iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v
    }

    /// 최근 지출 n건(역순).
    pub fn recent(&self, n: usize) -> Vec<&Expense> {
        self.items.iter().rev().take(n).collect()
    }
}

/// 이번 달 `(일평균, 월말 예상)`. 현 페이스가 유지된다는 가정의 추정.
/// total=이번 달 누적, day=경과 일수, days_in_month=이번 달 총 일수.
pub fn pace(total: i64, day: i64, days_in_month: i64) -> (i64, i64) {
    let day = day.max(1);
    (total / day, total * days_in_month / day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pace_projects_at_current_rate() {
        // 6일까지 146,500원 쓴 6월(30일) → 일평균 24,416, 월말 예상 732,500.
        assert_eq!(pace(146_500, 6, 30), (24_416, 732_500));
        // day=0이면 1로 보정(0 나눗셈 방지).
        assert_eq!(pace(10_000, 0, 31).0, 10_000);
    }

    fn mk() -> ExpenseStore {
        let mut s = ExpenseStore::default();
        s.items.push(Expense {
            id: 1,
            amount: 8000,
            category: "식비".into(),
            note: "점심".into(),
            date: "2026-06-01".into(),
        });
        s.items.push(Expense {
            id: 2,
            amount: 3000,
            category: "교통".into(),
            note: "".into(),
            date: "2026-06-01".into(),
        });
        s.items.push(Expense {
            id: 3,
            amount: 12000,
            category: "식비".into(),
            note: "저녁".into(),
            date: "2026-06-02".into(),
        });
        s
    }

    #[test]
    fn totals() {
        let s = mk();
        assert_eq!(s.total_on("2026-06-01"), 11000);
        assert_eq!(s.total_in_month("2026-06"), 23000);
    }

    #[test]
    fn category_breakdown() {
        let s = mk();
        let by = s.by_category_in_month("2026-06");
        assert_eq!(by[0], ("식비".to_string(), 20000)); // 식비가 최다
        assert_eq!(by[1], ("교통".to_string(), 3000));
    }

    #[test]
    fn won_format() {
        assert_eq!(won(8000), "8,000원");
        assert_eq!(won(1234567), "1,234,567원");
        assert_eq!(won(500), "500원");
    }
}
