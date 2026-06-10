//! 영속 메모리.
//!
//! "쓸수록 성장"의 토대. 에이전트가 사용자/환경에
//! 대해 배운 사실을 마크다운 파일에 누적 저장하고, 매 세션 시작 시 시스템
//! 프롬프트에 주입해 대화가 끊겨도 맥락이 이어지게 한다.
//!
//! 저장 위치: `~/.local/share/wonjang/memory.md` (플랫폼별 데이터 디렉터리).

use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct Memory {
    path: PathBuf,
}

impl Memory {
    /// 메모리 저장소를 연다(디렉터리가 없으면 생성).
    pub fn load() -> Result<Self> {
        let dir = dirs::data_dir()
            .context("데이터 디렉터리를 찾을 수 없습니다")?
            .join("wonjang");
        std::fs::create_dir_all(&dir).ok();
        Ok(Self {
            path: dir.join("memory.md"),
        })
    }

    /// 임의 경로로 메모리 저장소를 연다(테스트/고급 용도).
    #[allow(dead_code)]
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// 저장된 메모리 전체를 읽는다(없으면 빈 문자열).
    pub fn read(&self) -> String {
        std::fs::read_to_string(&self.path).unwrap_or_default()
    }

    /// 사실 한 줄을 추가한다(중복은 무시).
    /// 개행·연속 공백은 한 칸으로 접는다 — 여러 줄이 들어오면 둘째 줄부터 "- " 없는
    /// 고아 줄이 되어 파일 포맷이 깨지고 프롬프트 주입에서 유실된다(한 줄 계약 강제).
    pub fn append(&self, fact: &str) -> Result<()> {
        let fact = normalize(fact);
        let fact = fact.as_str();
        if fact.is_empty() {
            return Ok(());
        }
        let existing = self.read();
        // 동일 내용이 이미 있으면 추가하지 않는다.
        if existing
            .lines()
            .any(|l| l.trim_start_matches("- ").trim() == fact)
        {
            return Ok(());
        }
        let mut content = existing;
        if content.is_empty() {
            content.push_str("# 원장 메모리\n\n에이전트가 사용자/환경에 대해 기억하는 사실들.\n\n");
        }
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("- {fact}\n"));
        crate::util::atomic_write(&self.path, content.as_bytes())
            .with_context(|| format!("메모리를 저장할 수 없습니다: {}", self.path.display()))?;
        Ok(())
    }

    /// 기억한 사실 개수.
    pub fn count(&self) -> usize {
        self.read()
            .lines()
            .filter(|l| l.trim_start().starts_with("- "))
            .count()
    }

    /// 시스템 프롬프트에 주입할 메모리 블록(없으면 None).
    pub fn prompt_block(&self) -> Option<String> {
        let content = self.read();
        let facts: Vec<&str> = content
            .lines()
            .filter(|l| l.trim_start().starts_with("- "))
            .collect();
        if facts.is_empty() {
            None
        } else {
            Some(format!(
                "기억하고 있는 사실(이전 세션에서 학습):\n{}",
                facts.join("\n")
            ))
        }
    }
}

/// 사실 문자열 정규화(개행·연속 공백 → 한 칸). 저장(append)과 표시 메시지가
/// 같은 규칙을 쓰게 해, "기억했어요" 에코와 실제 저장 내용이 늘 일치한다.
pub fn normalize(fact: &str) -> String {
    fact.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_mem(name: &str) -> Memory {
        let mut p = std::env::temp_dir();
        p.push(format!("wonjang_test_{name}.md"));
        let _ = std::fs::remove_file(&p);
        Memory::with_path(p)
    }

    #[test]
    fn append_and_read() {
        let mem = temp_mem("append");
        mem.append("사용자는 Rust를 선호한다").unwrap();
        assert!(mem.read().contains("사용자는 Rust를 선호한다"));
    }

    #[test]
    fn multiline_fact_is_flattened_to_one_line() {
        // 개행 든 사실이 그대로 저장되면 둘째 줄이 고아가 되어 포맷·프롬프트가 깨진다.
        let mem = temp_mem("multiline");
        mem.append("줄하나\n줄둘\t  줄셋").unwrap();
        assert_eq!(mem.count(), 1);
        assert!(mem.read().contains("- 줄하나 줄둘 줄셋"));
        // 평탄화 후에도 중복 인식이 일관돼야 한다.
        mem.append("줄하나 줄둘 줄셋").unwrap();
        assert_eq!(mem.count(), 1);
        let block = mem.prompt_block().unwrap();
        assert!(block.contains("줄셋"), "프롬프트 주입에서 내용 유실");
    }

    #[test]
    fn count_tracks_facts() {
        let mem = temp_mem("count");
        assert_eq!(mem.count(), 0);
        mem.append("사실 A").unwrap();
        mem.append("사실 B").unwrap();
        mem.append("사실 A").unwrap(); // 중복은 늘지 않음
        assert_eq!(mem.count(), 2);
    }

    #[test]
    fn dedup_ignores_duplicates() {
        let mem = temp_mem("dedup");
        mem.append("동일 사실").unwrap();
        mem.append("동일 사실").unwrap();
        let count = mem.read().matches("동일 사실").count();
        assert_eq!(count, 1, "중복 사실은 한 번만 저장되어야 한다");
    }

    #[test]
    fn empty_fact_is_skipped() {
        let mem = temp_mem("empty");
        mem.append("   ").unwrap();
        assert!(mem.prompt_block().is_none());
    }

    #[test]
    fn prompt_block_lists_facts() {
        let mem = temp_mem("block");
        mem.append("사실 1").unwrap();
        mem.append("사실 2").unwrap();
        let block = mem.prompt_block().unwrap();
        assert!(block.contains("사실 1") && block.contains("사실 2"));
    }
}
