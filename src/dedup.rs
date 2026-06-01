//! 중복 파일 찾기 — 같은 파일이 여러 벌 쌓여 낭비되는 용량을 찾아준다.
//!
//! 먼저 같은 크기끼리 묶고, 그중 내용 해시(FNV-1a 64)가 같은 것만 중복으로 본다.
//! 읽기 전용이라 안전하다(파일을 지우지 않고 목록만 보여줌). 빈 파일(0바이트)은 뺀다.

use crate::diskusage;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// 같은 내용 파일들의 묶음.
pub struct DupGroup {
    pub size: u64,
    pub paths: Vec<PathBuf>,
}

impl DupGroup {
    /// 이 묶음에서 낭비되는 용량(중복본 수 × 크기).
    pub fn wasted(&self) -> u64 {
        self.size * (self.paths.len() as u64 - 1)
    }
}

/// 중복 탐색 결과.
pub struct DupResult {
    pub groups: Vec<DupGroup>,
    pub total_wasted: u64,
}

fn fnv1a_file(path: &Path) -> Option<u64> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Some(hash)
}

/// 폴더에서 내용이 같은 중복 파일을 찾는다.
pub fn find_duplicates(root: &Path) -> DupResult {
    // 1) 크기별로 묶기(0바이트 제외).
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for (path, size) in diskusage::collect_files(root) {
        if size > 0 {
            by_size.entry(size).or_default().push(path);
        }
    }

    // 2) 같은 크기가 2개 이상이면 해시로 확정.
    let mut groups: Vec<DupGroup> = Vec::new();
    let mut total_wasted = 0u64;
    for (size, paths) in by_size {
        if paths.len() < 2 {
            continue;
        }
        let mut by_hash: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for p in paths {
            if let Some(h) = fnv1a_file(&p) {
                by_hash.entry(h).or_default().push(p);
            }
        }
        for (_h, group_paths) in by_hash {
            if group_paths.len() >= 2 {
                let g = DupGroup {
                    size,
                    paths: group_paths,
                };
                total_wasted += g.wasted();
                groups.push(g);
            }
        }
    }

    // 낭비 용량 큰 순.
    groups.sort_by_key(|g| std::cmp::Reverse(g.wasted()));
    DupResult {
        groups,
        total_wasted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_identical_files() {
        let base = std::env::temp_dir().join("wonjang_dedup_test");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        // 같은 내용 2개 + 다른 내용 1개 + 빈 파일 2개.
        fs::write(base.join("a.txt"), b"hello world").unwrap();
        fs::write(base.join("sub/a_copy.txt"), b"hello world").unwrap();
        fs::write(base.join("b.txt"), b"different").unwrap();
        fs::write(base.join("e1"), b"").unwrap();
        fs::write(base.join("e2"), b"").unwrap();

        let r = find_duplicates(&base);
        assert_eq!(r.groups.len(), 1); // hello world 묶음 하나
        assert_eq!(r.groups[0].paths.len(), 2);
        assert_eq!(r.total_wasted, 11); // "hello world" 11바이트 한 벌 낭비

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn no_duplicates_is_empty() {
        let base = std::env::temp_dir().join("wonjang_dedup_test2");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("x"), b"aaa").unwrap();
        fs::write(base.join("y"), b"bbbb").unwrap();
        let r = find_duplicates(&base);
        assert!(r.groups.is_empty());
        assert_eq!(r.total_wasted, 0);
        let _ = fs::remove_dir_all(&base);
    }
}
