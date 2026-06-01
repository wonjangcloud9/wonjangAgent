//! 파일 비교(diff) — 두 텍스트 파일의 차이를 줄 단위로 보여준다.
//!
//! "이 설정 파일 뭐가 바뀌었지?", "두 버전 비교해줘" 같은 요청을 처리한다.
//! similar 크레이트(순수 Rust)로 LCS 기반 줄 비교를 한다.

use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};

/// 한 줄의 변경(추가/삭제/유지).
pub struct Line {
    pub tag: char, // '+', '-', ' '
    pub text: String,
}

/// 비교 결과.
pub struct DiffResult {
    pub lines: Vec<Line>,
    pub added: usize,
    pub removed: usize,
}

/// 두 텍스트의 줄 단위 diff를 만든다.
pub fn diff_text(old: &str, new: &str) -> DiffResult {
    let diff = TextDiff::from_lines(old, new);
    let mut lines = Vec::new();
    let mut added = 0;
    let mut removed = 0;
    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Delete => {
                removed += 1;
                '-'
            }
            ChangeTag::Insert => {
                added += 1;
                '+'
            }
            ChangeTag::Equal => ' ',
        };
        lines.push(Line {
            tag,
            text: change.value().trim_end_matches('\n').to_string(),
        });
    }
    DiffResult {
        lines,
        added,
        removed,
    }
}

/// 두 파일을 읽어 비교한다.
pub fn diff_files(a: &str, b: &str) -> Result<DiffResult> {
    let old = std::fs::read_to_string(a).with_context(|| format!("파일을 읽을 수 없어요: {a}"))?;
    let new = std::fs::read_to_string(b).with_context(|| format!("파일을 읽을 수 없어요: {b}"))?;
    Ok(diff_text(&old, &new))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_add_and_remove() {
        let old = "첫째\n둘째\n셋째\n";
        let new = "첫째\n둘째 수정\n셋째\n넷째\n";
        let d = diff_text(old, new);
        // "둘째" 삭제 + "둘째 수정"·"넷째" 추가.
        assert_eq!(d.removed, 1);
        assert_eq!(d.added, 2);
        assert!(d.lines.iter().any(|l| l.tag == '+' && l.text == "넷째"));
    }

    #[test]
    fn identical_has_no_changes() {
        let d = diff_text("같음\n동일\n", "같음\n동일\n");
        assert_eq!(d.added, 0);
        assert_eq!(d.removed, 0);
    }
}
