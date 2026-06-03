//! 간단 일기/저널 — 옵시디언 없이도 한 줄로 기록하고 다시 본다.
//!
//! 월별 마크다운 파일에 시각과 함께 덧붙인다. 키가 필요 없고 로컬에 영구
//! 저장된다(옵시디언 볼트가 설정돼 있으면 그쪽 노트도 함께 쓸 수 있다).
//!
//! 저장 위치: `~/.local/share/wonjang/journal/YYYY-MM.md`

use anyhow::{bail, Context, Result};
use chrono::Local;
use std::path::PathBuf;

fn journal_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang")
        .join("journal");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir)
}

/// 이번 달 파일 경로(YYYY-MM.md).
fn current_file() -> Result<PathBuf> {
    let month = Local::now().format("%Y-%m").to_string();
    Ok(journal_dir()?.join(format!("{month}.md")))
}

/// 한 줄(또는 여러 줄) 기록을 이번 달 파일에 덧붙인다.
pub fn add(text: &str) -> Result<PathBuf> {
    let text = text.trim();
    if text.is_empty() {
        bail!("기록할 내용을 입력하세요");
    }
    use std::io::Write;
    let path = current_file()?;
    let stamp = Local::now().format("%Y-%m-%d %H:%M");
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(f, "## {stamp}\n{text}\n")?;
    Ok(path)
}

/// 일기 한 건(시각 + 내용).
pub struct Entry {
    pub stamp: String,
    pub text: String,
}

/// 이번 달 기록을 최근 순으로 읽는다(없으면 빈 목록).
pub fn this_month() -> Result<Vec<Entry>> {
    let path = current_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let mut entries = parse(&content);
    entries.reverse(); // 최근이 위로
    Ok(entries)
}

/// `## ` 줄이 **일기 헤더**(시각 `YYYY-MM-DD HH:MM` 형식)인지 판별. 본문 속 마크다운 제목과 구분.
fn header_stamp(line: &str) -> Option<&str> {
    let t = line.strip_prefix("## ")?.trim();
    let b = t.as_bytes();
    // "2026-06-02 21:30" 형태: 위치별 구분자 + 앞 4자리 숫자.
    if t.len() >= 16
        && b.get(4) == Some(&b'-')
        && b.get(7) == Some(&b'-')
        && b.get(10) == Some(&b' ')
        && b.get(13) == Some(&b':')
        && t[..4].chars().all(|c| c.is_ascii_digit())
    {
        Some(t)
    } else {
        None
    }
}

/// 마크다운에서 `## 시각` + 본문 블록을 파싱한다.
/// 줄 단위로 헤더(시각 형식)만 새 항목 시작으로 보아, 본문에 `## 제목` 줄이 있어도 안 깨진다.
fn parse(md: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut cur: Option<(String, Vec<&str>)> = None;
    for line in md.lines() {
        if let Some(stamp) = header_stamp(line) {
            if let Some((s, body)) = cur.take() {
                out.push(Entry {
                    stamp: s,
                    text: body.join("\n").trim().to_string(),
                });
            }
            cur = Some((stamp.to_string(), Vec::new()));
        } else if let Some((_, body)) = cur.as_mut() {
            body.push(line);
        }
    }
    if let Some((s, body)) = cur.take() {
        out.push(Entry {
            stamp: s,
            text: body.join("\n").trim().to_string(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries() {
        let md = "## 2026-06-02 09:00\n첫 기록\n\n## 2026-06-02 21:30\n둘째 기록\n여러 줄\n";
        let e = parse(md);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].stamp, "2026-06-02 09:00");
        assert_eq!(e[0].text, "첫 기록");
        assert_eq!(e[1].text, "둘째 기록\n여러 줄");
    }

    #[test]
    fn body_with_markdown_heading_not_split() {
        // 본문에 '## ' 줄(마크다운 제목)이 있어도 가짜 항목으로 쪼개지지 않아야 한다.
        let md = "## 2026-06-02 09:00\n오늘 목표\n## 운동 하기\n끝\n\n## 2026-06-02 21:30\n둘째\n";
        let e = parse(md);
        assert_eq!(e.len(), 2, "본문 ## 때문에 잘못 쪼개짐");
        assert_eq!(e[0].text, "오늘 목표\n## 운동 하기\n끝");
        assert_eq!(e[1].text, "둘째");
    }

    #[test]
    fn empty_text_errors() {
        assert!(add("   ").is_err());
    }
}
