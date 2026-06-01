//! 파일 내용 검색 — 폴더 안 텍스트 파일에서 단어가 든 줄을 찾는다(grep과 비슷).
//!
//! "내 메모 중에 '계약' 들어간 거 찾아줘" 같은 요청을 처리한다. 바이너리·너무 큰
//! 파일은 건너뛰고, 대소문자 구분 없이 찾는다. 읽기 전용이라 안전하다.

use crate::diskusage;
use std::path::{Path, PathBuf};

/// 한 건의 검색 결과(파일·줄번호·줄 내용).
pub struct Match {
    pub file: PathBuf,
    pub line_no: usize,
    pub line: String,
}

/// 검색 결과 묶음.
pub struct SearchResult {
    pub matches: Vec<Match>,
    pub files_scanned: usize,
    pub truncated: bool,
}

const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024; // 5MB 넘는 파일은 건너뜀

/// 폴더에서 `query`가 든 줄을 찾는다(대소문자 무시). `max`는 결과 상한.
pub fn search(root: &Path, query: &str, max: usize) -> SearchResult {
    let needle = query.to_lowercase();
    let mut matches = Vec::new();
    let mut files_scanned = 0;
    let mut truncated = false;

    let mut files = diskusage::collect_files(root);
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (path, size) in files {
        if size == 0 || size > MAX_FILE_BYTES {
            continue;
        }
        // UTF-8로 못 읽으면(바이너리) 건너뛴다.
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        files_scanned += 1;
        for (i, line) in text.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                if matches.len() >= max {
                    truncated = true;
                    break;
                }
                matches.push(Match {
                    file: path.clone(),
                    line_no: i + 1,
                    line: line.trim().chars().take(200).collect(),
                });
            }
        }
        if truncated {
            break;
        }
    }

    SearchResult {
        matches,
        files_scanned,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_matching_lines() {
        let base = std::env::temp_dir().join("wonjang_search_test");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("a.txt"), "첫째 줄\n계약서 내용입니다\n셋째").unwrap();
        fs::write(base.join("sub/b.md"), "관련 없는 글\n계약 조건 메모").unwrap();
        fs::write(base.join("c.txt"), "전혀 다른 내용").unwrap();

        let r = search(&base, "계약", 100);
        assert_eq!(r.matches.len(), 2);
        assert_eq!(r.files_scanned, 3);
        assert!(r.matches.iter().any(|m| m.line.contains("계약서")));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn case_insensitive_and_limit() {
        let base = std::env::temp_dir().join("wonjang_search_test2");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("x.txt"), "TODO 1\ntodo 2\nTodo 3").unwrap();
        let r = search(&base, "todo", 2);
        assert_eq!(r.matches.len(), 2);
        assert!(r.truncated);
        let _ = fs::remove_dir_all(&base);
    }
}
