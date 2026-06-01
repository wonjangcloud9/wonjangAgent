//! 데이터 백업 — 원장이 쌓은 모든 데이터를 한 번에 백업한다.
//!
//! 약속·할일·디데이·가계부·습관·집중·즐겨찾기·시세알림·메모리·스킬·세션 등
//! `~/.local/share/wonjang/` 전체를 타임스탬프 폴더로 복사한다. 다른 기기로
//! 옮기거나 보관할 때 쓴다.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// 원장 데이터 디렉터리.
pub fn data_dir() -> Result<PathBuf> {
    Ok(dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang"))
}

/// 데이터를 `dest_dir/wonjang-backup-<timestamp>/`로 복사한다.
/// (백업 경로, 복사한 파일 수)를 반환.
pub fn backup(dest_dir: &Path, timestamp: &str) -> Result<(PathBuf, usize)> {
    let src = data_dir()?;
    if !src.exists() {
        bail!("백업할 데이터가 없습니다(아직 사용 기록이 없어요).");
    }
    let dest = dest_dir.join(format!("wonjang-backup-{timestamp}"));
    std::fs::create_dir_all(&dest)
        .with_context(|| format!("백업 폴더를 만들 수 없습니다: {}", dest.display()))?;
    let mut count = 0;
    copy_dir(&src, &dest, &mut count)?;
    Ok((dest, count))
}

/// 백업 폴더의 내용을 데이터 디렉터리로 복원한다(덮어씀). 복사한 파일 수 반환.
/// 호출자는 복원 전에 현재 데이터를 백업해 두는 것이 안전하다.
pub fn restore(backup_src: &Path, data_dest: &Path) -> Result<usize> {
    if !backup_src.exists() {
        bail!("백업 폴더를 찾을 수 없습니다: {}", backup_src.display());
    }
    std::fs::create_dir_all(data_dest)
        .with_context(|| format!("데이터 폴더를 만들 수 없습니다: {}", data_dest.display()))?;
    let mut count = 0;
    copy_dir(backup_src, data_dest, &mut count)?;
    Ok(count)
}

/// 디렉터리를 재귀적으로 복사한다.
fn copy_dir(src: &Path, dst: &Path, count: &mut usize) -> Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_dir(&path, &target, count)?;
        } else {
            std::fs::copy(&path, &target)?;
            *count += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_dir_recurses() {
        let mut base = std::env::temp_dir();
        base.push("wonjang_backup_test");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.json"), "1").unwrap();
        std::fs::write(src.join("sub/b.md"), "2").unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let mut count = 0;
        copy_dir(&src, &dst, &mut count).unwrap();
        assert_eq!(count, 2);
        assert!(dst.join("a.json").exists());
        assert!(dst.join("sub/b.md").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn restore_copies_into_dest() {
        let mut base = std::env::temp_dir();
        base.push("wonjang_restore_test");
        let _ = std::fs::remove_dir_all(&base);
        let bk = base.join("bk");
        let data = base.join("data");
        std::fs::create_dir_all(&bk).unwrap();
        std::fs::write(bk.join("reminders.json"), "[]").unwrap();

        let n = restore(&bk, &data).unwrap();
        assert_eq!(n, 1);
        assert!(data.join("reminders.json").exists());
        assert!(restore(&base.join("nope"), &data).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }
}
