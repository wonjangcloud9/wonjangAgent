//! 글자수 세기 — 자기소개서·SNS 등 글자수 제한 확인용.
//!
//! 한국에서 자소서는 보통 "공백 포함 1000자 이내"처럼 공백 포함/제외 기준이
//! 갈린다. 두 기준과 단어·줄·바이트 수를 함께 보여준다. 순수 계산이라 키가 없다.

/// 글자수 통계.
pub struct Count {
    pub chars_with_space: usize,    // 공백 포함 글자수
    pub chars_without_space: usize, // 공백 제외 글자수
    pub words: usize,               // 단어 수(공백 기준)
    pub lines: usize,               // 줄 수
    pub bytes: usize,               // UTF-8 바이트 수
}

/// 문자열의 글자수 통계를 낸다.
pub fn count(text: &str) -> Count {
    let chars_with_space = text.chars().count();
    let chars_without_space = text.chars().filter(|c| !c.is_whitespace()).count();
    let words = text.split_whitespace().count();
    let lines = if text.is_empty() {
        0
    } else {
        text.lines().count().max(1)
    };
    Count {
        chars_with_space,
        chars_without_space,
        words,
        lines,
        bytes: text.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_korean_with_and_without_space() {
        // "안녕 하세요" → 공백 포함 6, 제외 5.
        let c = count("안녕 하세요");
        assert_eq!(c.chars_with_space, 6);
        assert_eq!(c.chars_without_space, 5);
        assert_eq!(c.words, 2);
    }

    #[test]
    fn korean_char_is_one_count() {
        // 한글 한 글자는 1자(바이트는 3).
        let c = count("가");
        assert_eq!(c.chars_with_space, 1);
        assert_eq!(c.bytes, 3);
    }

    #[test]
    fn counts_lines() {
        let c = count("첫줄\n둘째줄\n셋째줄");
        assert_eq!(c.lines, 3);
    }

    #[test]
    fn empty_is_zero() {
        let c = count("");
        assert_eq!(c.chars_with_space, 0);
        assert_eq!(c.lines, 0);
        assert_eq!(c.words, 0);
    }
}
