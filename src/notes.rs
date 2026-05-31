//! 옵시디언(로컬 마크다운 볼트) 통합.
//!
//! 한국 사용자가 많이 쓰는 옵시디언 볼트를 24시간 비서가 읽고/검색하고/기록한다.
//! 볼트 경로는 설정(`obsidian_vault`)이나 `WONJANG_OBSIDIAN_VAULT`로 지정한다.
//!
//! 보안: 모든 경로는 볼트 안으로 제한해 `../` 등으로 볼트 밖을 건드리지 못하게 한다.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// 설정된 볼트 경로(`~` 확장 포함). 비활성이면 None.
pub fn vault_path(raw: &str) -> Option<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    Some(expand_home(raw))
}

/// 앞쪽 `~`를 홈 디렉터리로 확장.
fn expand_home(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if p == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(p)
}

/// 볼트 기준 상대 경로를 안전하게 절대 경로로 변환한다(볼트 밖 탈출 차단).
pub fn resolve(vault: &Path, rel: &str) -> Result<PathBuf> {
    let rel = rel.trim().trim_start_matches('/');
    // 컴포넌트 단위로 `..` 금지.
    for comp in Path::new(rel).components() {
        if matches!(comp, std::path::Component::ParentDir) {
            bail!("볼트 밖 경로(..)는 허용되지 않습니다: {rel}");
        }
    }
    let mut path = vault.join(rel);
    // .md 확장자 자동 보정(디렉터리가 아니면).
    if path.extension().is_none() {
        path.set_extension("md");
    }
    Ok(path)
}

/// 볼트의 모든 마크다운 파일 경로(볼트 기준 상대).
pub fn list_markdown(vault: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    walk_md(vault, vault, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_md(vault: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // 숨김 폴더(.obsidian, .trash, .git)는 건너뛴다.
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk_md(vault, &path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Ok(rel) = path.strip_prefix(vault) {
                out.push(rel.to_string_lossy().to_string());
            }
        }
    }
    Ok(())
}

/// 검색 결과 한 건.
pub struct NoteHit {
    pub file: String,
    pub line_no: usize,
    pub line: String,
}

/// 볼트 전체에서 질의어(대소문자 무시)를 포함하는 줄을 찾는다.
pub fn search(vault: &Path, query: &str, limit: usize) -> Result<Vec<NoteHit>> {
    let q = query.to_lowercase();
    let mut hits = Vec::new();
    for rel in list_markdown(vault)? {
        if hits.len() >= limit {
            break;
        }
        let path = vault.join(&rel);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&q) {
                hits.push(NoteHit {
                    file: rel.clone(),
                    line_no: i + 1,
                    line: line.trim().chars().take(160).collect(),
                });
                if hits.len() >= limit {
                    break;
                }
            }
        }
    }
    Ok(hits)
}

/// 노트에 내용을 덧붙인다(없으면 생성, 상위 폴더 자동 생성).
pub fn append(vault: &Path, rel: &str, content: &str) -> Result<PathBuf> {
    use std::io::Write;
    let path = resolve(vault, rel)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("노트를 열 수 없습니다: {}", path.display()))?;
    // 기존 내용이 있고 개행으로 끝나지 않으면 줄바꿈 추가.
    let needs_nl = std::fs::metadata(&path)
        .map(|m| m.len() > 0)
        .unwrap_or(false);
    if needs_nl {
        writeln!(file)?;
    }
    writeln!(file, "{content}")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_vault(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("wonjang_vault_{name}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn resolve_blocks_traversal() {
        let v = tmp_vault("resolve");
        assert!(resolve(&v, "../escape").is_err());
        let ok = resolve(&v, "노트/오늘").unwrap();
        assert!(ok.ends_with("노트/오늘.md"));
    }

    #[test]
    fn append_and_search() {
        let v = tmp_vault("append");
        append(&v, "일지/2026-05-31", "오늘 원장 에이전트를 개발했다").unwrap();
        append(&v, "일지/2026-05-31", "버스 시간도 확인했다").unwrap();
        let hits = search(&v, "버스", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].line.contains("버스 시간"));
        assert_eq!(hits[0].file, "일지/2026-05-31.md");
    }

    #[test]
    fn list_skips_hidden() {
        let v = tmp_vault("list");
        append(&v, "a", "x").unwrap();
        std::fs::create_dir_all(v.join(".obsidian")).unwrap();
        std::fs::write(v.join(".obsidian/app.md"), "내부").unwrap();
        let files = list_markdown(&v).unwrap();
        assert_eq!(files, vec!["a.md"]);
    }

    #[test]
    fn vault_path_expands_home() {
        assert!(vault_path("").is_none());
        let p = vault_path("~/Obsidian").unwrap();
        assert!(!p.to_string_lossy().starts_with('~'));
    }
}
