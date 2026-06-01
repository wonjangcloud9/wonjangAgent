//! 한글 초성 추출 — "안녕하세요" → "ㅇㄴㅎㅅㅇ".
//!
//! 초성 퀴즈·초성 검색 등에 쓴다. 한글 음절(U+AC00~U+D7A3)을 유니코드
//! 규칙으로 분해하므로 정확하다. 한글이 아닌 문자는 그대로 둔다.

/// 19개 초성(가나다 순).
const CHOSEONG: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];

const HANGUL_BASE: u32 = 0xAC00;
const HANGUL_LAST: u32 = 0xD7A3;
// 한 초성당 음절 수 = 중성(21) × 종성(28).
const CHO_BLOCK: u32 = 21 * 28;

/// 한 글자의 초성을 반환한다. 완성형 한글이 아니면 그대로 반환.
pub fn choseong_of(c: char) -> char {
    let code = c as u32;
    if (HANGUL_BASE..=HANGUL_LAST).contains(&code) {
        let idx = ((code - HANGUL_BASE) / CHO_BLOCK) as usize;
        CHOSEONG[idx]
    } else {
        c
    }
}

/// 문자열 전체의 초성을 추출한다.
pub fn choseong(text: &str) -> String {
    text.chars().map(choseong_of).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_choseong() {
        assert_eq!(choseong("안녕하세요"), "ㅇㄴㅎㅅㅇ");
        assert_eq!(choseong("원장"), "ㅇㅈ");
    }

    #[test]
    fn keeps_non_hangul() {
        assert_eq!(choseong("AI 비서"), "AI ㅂㅅ");
        assert_eq!(choseong("123"), "123");
    }

    #[test]
    fn handles_double_consonants() {
        // 깍두기 → ㄲㄷㄱ.
        assert_eq!(choseong("깍두기"), "ㄲㄷㄱ");
    }

    #[test]
    fn boundary_syllables() {
        // 가(첫 음절)=ㄱ, 힣(마지막 음절)=ㅎ.
        assert_eq!(choseong("가"), "ㄱ");
        assert_eq!(choseong("힣"), "ㅎ");
    }
}
