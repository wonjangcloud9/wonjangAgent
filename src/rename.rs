//! 파일 이름 일괄 변경 — 폴더 안 파일명의 특정 문자를 한 번에 치환한다.
//!
//! 예: `IMG_` → `여행_`. 되돌리기 어려운 작업이라 호출 측에서 미리보기 후
//! 명시적으로 `execute`를 부를 때만 실제로 바꾼다. 최상위 파일만 다루고,
//! 숨김 파일·폴더는 제외한다. 이름이 겹치면 " (n)"을 붙여 덮어쓰지 않는다.

use crate::organize;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// 이름 변경 한 건.
pub struct Rename {
    pub from: PathBuf,
    pub to_name: String,
}

/// `find`를 `replace`로 바꾼 이름 변경 계획을 만든다(실제로 바꾸지 않음).
pub fn plan(dir: &Path, find: &str, replace: &str) -> Result<Vec<Rename>> {
    if find.is_empty() {
        return Err(anyhow!("찾을 문자열을 입력하세요"));
    }
    if !dir.is_dir() {
        return Err(anyhow!("폴더가 아니에요: {}", dir.display()));
    }
    let mut plans = Vec::new();
    for entry in std::fs::read_dir(dir)?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() || !name.contains(find) {
            continue;
        }
        let new_name = name.replace(find, replace);
        if new_name.is_empty() || new_name == name {
            continue;
        }
        plans.push(Rename {
            from: entry.path(),
            to_name: new_name,
        });
    }
    Ok(plans)
}

/// 계획대로 실제 이름을 바꾼다. 바꾼 개수를 반환.
pub fn execute(dir: &Path, plans: &[Rename]) -> Result<usize> {
    let mut count = 0;
    for r in plans {
        let target = organize::unique_target(dir, &r.to_name);
        std::fs::rename(&r.from, &target)?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn plans_only_matching_files() {
        let base = std::env::temp_dir().join("wonjang_rename_test");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("IMG_1.jpg"), b"x").unwrap();
        fs::write(base.join("IMG_2.jpg"), b"x").unwrap();
        fs::write(base.join("doc.pdf"), b"x").unwrap();

        let plans = plan(&base, "IMG_", "여행_").unwrap();
        assert_eq!(plans.len(), 2);
        assert!(plans.iter().all(|p| p.to_name.starts_with("여행_")));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn executes_rename() {
        let base = std::env::temp_dir().join("wonjang_rename_exec");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("old_a.txt"), b"x").unwrap();

        let plans = plan(&base, "old", "new").unwrap();
        assert_eq!(execute(&base, &plans).unwrap(), 1);
        assert!(base.join("new_a.txt").exists());
        assert!(!base.join("old_a.txt").exists());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn empty_find_errors() {
        let base = std::env::temp_dir().join("wonjang_rename_empty");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        assert!(plan(&base, "", "x").is_err());
        let _ = fs::remove_dir_all(&base);
    }
}
