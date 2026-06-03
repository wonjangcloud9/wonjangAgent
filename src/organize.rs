//! 폴더 자동 분류 — 다운로드 폴더처럼 뒤섞인 파일을 종류별 하위 폴더로 옮긴다.
//!
//! 파일을 옮기는 작업이라 **기본은 미리보기**다(무엇을 어디로 옮길지만 보여줌).
//! 실제 이동은 호출 측에서 `execute`를 명시적으로 부를 때만 일어난다.
//! 최상위 파일만 다루고(하위 폴더는 건드리지 않음), 숨김 파일·이미 분류된
//! 폴더는 제외한다.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// 분류 카테고리(하위 폴더 이름). 알 수 없는 확장자는 "기타".
pub fn category(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "bmp" | "svg" | "tiff" => "이미지",
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" => "동영상",
        "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" => "음악",
        "pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx" | "hwp" | "hwpx" | "txt"
        | "md" | "csv" | "rtf" => "문서",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "압축",
        "dmg" | "pkg" | "exe" | "msi" | "deb" | "app" => "설치파일",
        _ => "기타",
    }
}

/// 옮길 계획 한 건.
pub struct Move {
    pub from: PathBuf,
    pub category: &'static str,
}

/// 폴더의 최상위 파일들을 훑어 이동 계획을 만든다(실제 이동은 안 함).
pub fn plan(dir: &Path) -> Result<Vec<Move>> {
    if !dir.is_dir() {
        return Err(anyhow!("폴더가 아니에요: {}", dir.display()));
    }
    let mut plans = Vec::new();
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // 숨김 파일 제외.
        if name.starts_with('.') {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        // 폴더는 건드리지 않는다(이미 만든 분류 폴더 포함).
        if !meta.is_file() {
            continue;
        }
        plans.push(Move {
            from: path.clone(),
            category: category(&path),
        });
    }
    plans.sort_by(|a, b| a.category.cmp(b.category));
    Ok(plans)
}

/// 이름 충돌 시 " (1)", " (2)"를 붙여 비어 있는 경로를 찾는다.
pub fn unique_target(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    let ext = path.extension().and_then(|e| e.to_str());
    for i in 1..10_000 {
        let name = match ext {
            Some(e) => format!("{stem} ({i}).{e}"),
            None => format!("{stem} ({i})"),
        };
        let candidate = dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(file_name)
}

/// 계획대로 실제 이동한다. 옮긴 파일 수를 반환.
pub fn execute(dir: &Path, plans: &[Move]) -> Result<usize> {
    let mut moved = 0;
    for m in plans {
        let target_dir = dir.join(m.category);
        std::fs::create_dir_all(&target_dir)?;
        let file_name = m
            .from
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("파일 이름을 읽을 수 없어요"))?;
        let target = unique_target(&target_dir, file_name);
        std::fs::rename(&m.from, &target)?;
        moved += 1;
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn categorizes_by_extension() {
        assert_eq!(category(Path::new("a.JPG")), "이미지");
        assert_eq!(category(Path::new("b.mp4")), "동영상");
        assert_eq!(category(Path::new("c.pdf")), "문서");
        assert_eq!(category(Path::new("d.zip")), "압축");
        assert_eq!(category(Path::new("e.unknownext")), "기타");
        assert_eq!(category(Path::new("noext")), "기타");
    }

    #[test]
    fn plan_skips_dirs_and_hidden() {
        let base = std::env::temp_dir().join("wonjang_org_test");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("기존폴더")).unwrap();
        fs::write(base.join("photo.png"), b"x").unwrap();
        fs::write(base.join(".hidden"), b"x").unwrap();
        fs::write(base.join("report.pdf"), b"x").unwrap();

        let plans = plan(&base).unwrap();
        assert_eq!(plans.len(), 2); // 폴더·숨김 제외
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn execute_moves_into_category_folders() {
        let base = std::env::temp_dir().join("wonjang_org_exec");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("a.png"), b"x").unwrap();
        fs::write(base.join("b.pdf"), b"x").unwrap();

        let plans = plan(&base).unwrap();
        let moved = execute(&base, &plans).unwrap();
        assert_eq!(moved, 2);
        assert!(base.join("이미지/a.png").exists());
        assert!(base.join("문서/b.pdf").exists());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn unique_target_avoids_overwrite() {
        let base = std::env::temp_dir().join("wonjang_org_uniq");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("a.txt"), b"x").unwrap();
        let t = unique_target(&base, "a.txt");
        assert_eq!(t, base.join("a (1).txt"));
        let _ = fs::remove_dir_all(&base);
    }

    // 데이터 안전 핵심 불변식: 분류 폴더에 같은 이름 파일이 이미 있어도
    // execute가 그것을 덮어쓰지 않고(원본 내용 보존) 새 파일은 " (N)"으로 옮긴다.
    // (execute가 unique_target을 거치지 않게 바뀌면 무성 데이터 손실 → 이 테스트가 막는다)
    #[test]
    fn execute_collision_preserves_existing_content() {
        let base = std::env::temp_dir().join("wonjang_org_collide");
        let _ = fs::remove_dir_all(&base);
        // 이미 분류된 문서/계약서.pdf(원본 A)가 있고, 루트에 동명(내용 B) 파일.
        fs::create_dir_all(base.join("문서")).unwrap();
        fs::write(base.join("문서/계약서.pdf"), b"AAA-original").unwrap();
        fs::write(base.join("계약서.pdf"), b"BBB-incoming").unwrap();

        let plans = plan(&base).unwrap();
        let moved = execute(&base, &plans).unwrap();
        assert_eq!(moved, 1);
        // 원본 A는 그대로, 새 파일 B는 " (1)"로 들어와 둘 다 보존.
        assert_eq!(
            fs::read(base.join("문서/계약서.pdf")).unwrap(),
            b"AAA-original",
            "기존 파일이 덮어써졌다 — 데이터 손실!"
        );
        assert_eq!(
            fs::read(base.join("문서/계약서 (1).pdf")).unwrap(),
            b"BBB-incoming"
        );
        let _ = fs::remove_dir_all(&base);
    }
}
