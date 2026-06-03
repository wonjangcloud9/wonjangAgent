//! 폴더·파일 압축(zip)과 해제 — 자주 쓰는 로컬 비서 작업.
//!
//! 압축은 새 .zip을 만들고(기존 파일을 건드리지 않음), 해제는 지정 폴더(기본은
//! zip 이름의 새 폴더)로 풀어 덮어쓰기를 피한다. Deflate 압축을 쓴다.

use anyhow::{anyhow, bail, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

/// 폴더 안 파일을 (절대경로, zip 내부 상대경로)로 모은다.
fn collect(base: &Path, root_label: &Path, out: &mut Vec<(PathBuf, String)>) {
    let entries = match std::fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            collect(&path, root_label, out);
        } else if meta.is_file() {
            // zip 내부 경로: root_label(폴더 이름) 기준 상대경로.
            if let Ok(rel) = path.strip_prefix(root_label.parent().unwrap_or(Path::new(""))) {
                out.push((path.clone(), rel.to_string_lossy().replace('\\', "/")));
            }
        }
    }
}

/// 소스(폴더/파일)들을 하나의 zip으로 압축한다. 담은 파일 수를 반환.
pub fn create_zip(sources: &[PathBuf], output: &Path) -> Result<usize> {
    if output.exists() {
        bail!(
            "이미 있는 파일이에요: {} (다른 이름을 쓰세요)",
            output.display()
        );
    }
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for src in sources {
        if !src.exists() {
            bail!("없는 경로예요: {}", src.display());
        }
        if src.is_dir() {
            collect(src, src, &mut files);
        } else {
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .ok_or_else(|| anyhow!("파일 이름을 읽을 수 없어요"))?;
            files.push((src.clone(), name));
        }
    }
    if files.is_empty() {
        bail!("압축할 파일이 없어요");
    }

    let zip_file = File::create(output)?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut buf = Vec::new();
    for (abs, rel) in &files {
        zip.start_file(rel, options)?;
        buf.clear();
        File::open(abs)?.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
    }
    zip.finish()?;
    Ok(files.len())
}

/// zip 엔트리 파일명을 사람이 읽게 디코드한다(UTF-8이면 그대로, 아니면 CP949).
/// 윈도우에서 만든 한국 zip은 파일명이 CP949라 다른 도구·맥에선 깨져 보인다.
pub fn decode_zip_name(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::EUC_KR.decode(raw).0.into_owned(),
    }
}

/// 디코드된 zip 내부 경로를 zip-slip 안전한 상대경로로 정제한다(절대경로·`..` 차단).
fn safe_relative(name: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for comp in name.split(['/', '\\']) {
        match comp {
            "" | "." => continue,
            ".." => return None, // 경로 탈출 차단
            c => out.push(c),
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

/// zip을 대상 폴더로 푼다. 푼 파일 수를 반환.
pub fn extract_zip(zip_path: &Path, dest: &Path) -> Result<usize> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| anyhow!("zip을 열 수 없어요: {e}"))?;
    std::fs::create_dir_all(dest)?;
    let mut count = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        // 한글 파일명(CP949)을 제대로 디코드한 뒤 zip-slip 안전 경로로 정제.
        let name = decode_zip_name(entry.name_raw());
        let rel = match safe_relative(&name) {
            Some(p) => p,
            None => continue,
        };
        let target = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&target)?;
            std::io::copy(&mut entry, &mut out)?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_and_unzip_roundtrip() {
        let base = std::env::temp_dir().join("wonjang_zip_test");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("data/sub")).unwrap();
        std::fs::write(base.join("data/a.txt"), b"hello").unwrap();
        std::fs::write(base.join("data/sub/b.txt"), b"world").unwrap();

        let zip_path = base.join("out.zip");
        let n = create_zip(&[base.join("data")], &zip_path).unwrap();
        assert_eq!(n, 2);
        assert!(zip_path.exists());

        let dest = base.join("unzipped");
        let extracted = extract_zip(&zip_path, &dest).unwrap();
        assert_eq!(extracted, 2);
        assert_eq!(
            std::fs::read_to_string(dest.join("data/a.txt")).unwrap(),
            "hello"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("data/sub/b.txt")).unwrap(),
            "world"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn refuses_existing_output() {
        let base = std::env::temp_dir().join("wonjang_zip_exist");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("a.txt"), b"x").unwrap();
        let out = base.join("a.zip");
        std::fs::write(&out, b"existing").unwrap();
        assert!(create_zip(&[base.join("a.txt")], &out).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }
}
