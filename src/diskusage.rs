//! 디스크 용량 분석 — "내 컴퓨터 어디가 꽉 찼지?"를 대신 찾아준다.
//!
//! 폴더를 재귀적으로 훑어 총 용량·파일 수, 가장 큰 파일과 하위 폴더를 보여준다.
//! 읽기 전용이라 안전하다(파일을 옮기거나 지우지 않음). 심볼릭 링크는 따라가지
//! 않고, 권한 오류 등은 건너뛴다.

use std::fs;
use std::path::{Path, PathBuf};

/// 분석 결과.
pub struct Usage {
    pub total: u64,
    pub file_count: u64,
    /// (파일 경로, 크기) 큰 순.
    pub largest_files: Vec<(PathBuf, u64)>,
    /// (바로 아래 하위 폴더 경로, 크기) 큰 순.
    pub largest_dirs: Vec<(PathBuf, u64)>,
}

/// 한 폴더를 재귀로 합산. (총 바이트, 파일 수)를 반환하고, 파일들을 `files`에 모은다.
fn walk(dir: &Path, files: &mut Vec<(PathBuf, u64)>) -> (u64, u64) {
    let mut total = 0u64;
    let mut count = 0u64;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return (0, 0),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // 심볼릭 링크는 따라가지 않는다(무한 루프·중복 방지).
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            let (sub, c) = walk(&path, files);
            total += sub;
            count += c;
        } else if meta.is_file() {
            let size = meta.len();
            total += size;
            count += 1;
            files.push((path, size));
        }
    }
    (total, count)
}

/// 경로를 분석한다. `top_n`은 보여줄 상위 항목 수.
pub fn analyze(root: &Path, top_n: usize) -> std::io::Result<Usage> {
    if !root.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "경로가 없습니다",
        ));
    }

    // 바로 아래 하위 폴더별 크기.
    let mut largest_dirs: Vec<(PathBuf, u64)> = Vec::new();
    let mut all_files: Vec<(PathBuf, u64)> = Vec::new();
    let mut total = 0u64;
    let mut file_count = 0u64;

    if root.is_file() {
        let size = fs::metadata(root)?.len();
        return Ok(Usage {
            total: size,
            file_count: 1,
            largest_files: vec![(root.to_path_buf(), size)],
            largest_dirs: Vec::new(),
        });
    }

    for entry in fs::read_dir(root)?.flatten() {
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            let (sub, c) = walk(&path, &mut all_files);
            total += sub;
            file_count += c;
            largest_dirs.push((path, sub));
        } else if meta.is_file() {
            let size = meta.len();
            total += size;
            file_count += 1;
            all_files.push((path, size));
        }
    }

    all_files.sort_by_key(|x| std::cmp::Reverse(x.1));
    all_files.truncate(top_n);
    largest_dirs.sort_by_key(|x| std::cmp::Reverse(x.1));
    largest_dirs.truncate(top_n);

    Ok(Usage {
        total,
        file_count,
        largest_files: all_files,
        largest_dirs,
    })
}

/// 바이트를 사람이 읽기 좋은 단위로(예: 1.2GB).
pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}B")
    } else {
        format!("{size:.1}{}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_readable_sizes() {
        assert_eq!(human(512), "512B");
        assert_eq!(human(1024), "1.0KB");
        assert_eq!(human(1536), "1.5KB");
        assert_eq!(human(1024 * 1024), "1.0MB");
        assert_eq!(human(3 * 1024 * 1024 * 1024), "3.0GB");
    }

    #[test]
    fn analyzes_a_temp_dir() {
        let base = std::env::temp_dir().join("wonjang_du_test");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("a.txt"), vec![0u8; 100]).unwrap();
        fs::write(base.join("sub/b.txt"), vec![0u8; 300]).unwrap();

        let u = analyze(&base, 10).unwrap();
        assert_eq!(u.total, 400);
        assert_eq!(u.file_count, 2);
        // 가장 큰 파일은 b.txt(300).
        assert_eq!(u.largest_files[0].1, 300);
        // 하위 폴더 sub은 300.
        assert_eq!(u.largest_dirs[0].1, 300);

        let _ = fs::remove_dir_all(&base);
    }
}
