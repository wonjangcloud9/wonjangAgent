//! 한글 → 영문 타자 변환 — "안녕" → "dkssud".
//!
//! 두벌식 자판 기준으로, 한글을 영문 모드에서 치는 키 순서로 바꾼다.
//! 완성형 한글(U+AC00~U+D7A3)을 초성·중성·종성으로 분해해 각 자모의 키를
//! 이어 붙인다. 한글이 아닌 문자는 그대로 둔다. 외부 의존성·키가 없다.

const HANGUL_BASE: u32 = 0xAC00;
const HANGUL_LAST: u32 = 0xD7A3;

// 초성 19개의 두벌식 키.
const CHO: [&str; 19] = [
    "r", "R", "s", "e", "E", "f", "a", "q", "Q", "t", "T", "d", "w", "W", "c", "z", "x", "v", "g",
];
// 중성 21개의 키(겹모음은 두 키).
const JUNG: [&str; 21] = [
    "k", "o", "i", "O", "j", "p", "u", "P", "h", "hk", "ho", "hl", "y", "n", "nj", "np", "nl", "b",
    "m", "ml", "l",
];
// 종성 28개의 키(0=받침 없음, 겹받침은 두 키).
const JONG: [&str; 28] = [
    "", "r", "R", "rt", "s", "sw", "sg", "e", "f", "fr", "fa", "fq", "ft", "fx", "fv", "fg", "a",
    "q", "qt", "t", "T", "d", "w", "c", "z", "x", "v", "g",
];

/// 한 글자를 두벌식 키 순서로. 완성형 한글이 아니면 그대로.
fn char_to_keys(c: char, out: &mut String) {
    let code = c as u32;
    if (HANGUL_BASE..=HANGUL_LAST).contains(&code) {
        let offset = code - HANGUL_BASE;
        let cho = (offset / 588) as usize;
        let jung = ((offset % 588) / 28) as usize;
        let jong = (offset % 28) as usize;
        out.push_str(CHO[cho]);
        out.push_str(JUNG[jung]);
        out.push_str(JONG[jong]);
    } else {
        out.push(c);
    }
}

/// 문자열 전체를 영문 타자로 변환한다.
pub fn han_to_eng(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        char_to_keys(c, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_words() {
        assert_eq!(han_to_eng("안녕"), "dkssud");
        assert_eq!(han_to_eng("한글"), "gksrmf");
        assert_eq!(han_to_eng("원장"), "dnjswkd");
    }

    #[test]
    fn compound_vowel_and_final() {
        // 과 = ㄱ(r) + ㅘ(hk) → "rhk".
        assert_eq!(han_to_eng("과"), "rhk");
        // 닭 = ㄷ(e) + ㅏ(k) + ㄺ(fr) → "ekfr".
        assert_eq!(han_to_eng("닭"), "ekfr");
    }

    #[test]
    fn keeps_non_hangul() {
        assert_eq!(han_to_eng("AI 비서"), "AI qltj");
        assert_eq!(han_to_eng("hello"), "hello");
    }
}
