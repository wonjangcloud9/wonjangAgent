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

// ── 영문 → 한글(조합) ───────────────────────────────────────────────

// 받침 없는 호환 자음/모음(낱자 출력용).
const COMPAT_CHO: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];
const COMPAT_JUNG: [char; 21] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ',
    'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];

/// 자음 키 → (초성 인덱스, 종성 인덱스 Option).
fn cons_of(key: char) -> Option<(usize, Option<usize>)> {
    let v = match key {
        'r' => (0, Some(1)),
        'R' => (1, Some(2)),
        's' => (2, Some(4)),
        'e' => (3, Some(7)),
        'E' => (4, None),
        'f' => (5, Some(8)),
        'a' => (6, Some(16)),
        'q' => (7, Some(17)),
        'Q' => (8, None),
        't' => (9, Some(19)),
        'T' => (10, Some(20)),
        'd' => (11, Some(21)),
        'w' => (12, Some(22)),
        'W' => (13, None),
        'c' => (14, Some(23)),
        'z' => (15, Some(24)),
        'x' => (16, Some(25)),
        'v' => (17, Some(26)),
        'g' => (18, Some(27)),
        _ => return None,
    };
    Some(v)
}

/// 모음 키 → 중성 인덱스.
fn vowel_of(key: char) -> Option<usize> {
    let v = match key {
        'k' => 0,
        'o' => 1,
        'i' => 2,
        'O' => 3,
        'j' => 4,
        'p' => 5,
        'u' => 6,
        'P' => 7,
        'h' => 8,
        'y' => 12,
        'n' => 13,
        'b' => 17,
        'm' => 18,
        'l' => 20,
        _ => return None,
    };
    Some(v)
}

/// 두 중성을 겹모음으로 합칠 수 있으면 그 인덱스.
fn vowel_combine(a: usize, b: usize) -> Option<usize> {
    match (a, b) {
        (8, 0) => Some(9),
        (8, 1) => Some(10),
        (8, 20) => Some(11),
        (13, 4) => Some(14),
        (13, 5) => Some(15),
        (13, 20) => Some(16),
        (18, 20) => Some(19),
        _ => None,
    }
}

/// 종성 + 자음을 겹받침으로 합칠 수 있으면 그 인덱스.
fn jong_combine(a: usize, cons_jong: usize) -> Option<usize> {
    match (a, cons_jong) {
        (1, 19) => Some(3),
        (4, 22) => Some(5),
        (4, 27) => Some(6),
        (8, 1) => Some(9),
        (8, 16) => Some(10),
        (8, 17) => Some(11),
        (8, 19) => Some(12),
        (8, 25) => Some(13),
        (8, 26) => Some(14),
        (8, 27) => Some(15),
        (17, 19) => Some(18),
        _ => None,
    }
}

/// 겹받침을 (앞 종성, 뒤 자음의 초성)으로 나눈다. 홑받침이면 None.
fn jong_split(jong: usize) -> Option<(usize, usize)> {
    let v = match jong {
        3 => (1, 9),
        5 => (4, 12),
        6 => (4, 18),
        9 => (8, 0),
        10 => (8, 6),
        11 => (8, 7),
        12 => (8, 9),
        13 => (8, 16),
        14 => (8, 17),
        15 => (8, 18),
        18 => (17, 9),
        _ => return None,
    };
    Some(v)
}

/// 홑받침 종성 인덱스 → 초성 인덱스(받침이 다음 글자의 초성으로 넘어갈 때).
fn jong_to_cho(jong: usize) -> Option<usize> {
    let v = match jong {
        1 => 0,
        2 => 1,
        4 => 2,
        7 => 3,
        8 => 5,
        16 => 6,
        17 => 7,
        19 => 9,
        20 => 10,
        21 => 11,
        22 => 12,
        23 => 14,
        24 => 15,
        25 => 16,
        26 => 17,
        27 => 18,
        _ => return None,
    };
    Some(v)
}

#[derive(Default, Clone, Copy)]
struct Syl {
    cho: Option<usize>,
    jung: Option<usize>,
    jong: usize, // 0 = 받침 없음
}

impl Syl {
    /// 현재 음절을 글자로 만들어 out에 붙인다.
    fn flush(&self, out: &mut String) {
        match (self.cho, self.jung) {
            (Some(cho), Some(jung)) => {
                let code = 0xAC00 + (cho * 21 + jung) * 28 + self.jong;
                if let Some(c) = char::from_u32(code as u32) {
                    out.push(c);
                }
            }
            (Some(cho), None) => out.push(COMPAT_CHO[cho]),
            (None, Some(jung)) => out.push(COMPAT_JUNG[jung]),
            (None, None) => {}
        }
    }
}

/// 영문 타자(잘못 친 한글)를 한글로 복원한다.
pub fn eng_to_han(text: &str) -> String {
    let mut out = String::new();
    let mut cur = Syl::default();

    for ch in text.chars() {
        // Q/W/E/R/T/O/P 외의 대문자는 소문자로 취급(두벌식).
        let key = if matches!(ch, 'Q' | 'W' | 'E' | 'R' | 'T' | 'O' | 'P') {
            ch
        } else if ch.is_ascii_uppercase() {
            ch.to_ascii_lowercase()
        } else {
            ch
        };

        if let Some(jung) = vowel_of(key) {
            handle_vowel(&mut cur, jung, &mut out);
        } else if let Some((cho, jong)) = cons_of(key) {
            handle_consonant(&mut cur, cho, jong, &mut out);
        } else {
            // 한글 자모가 아닌 문자(공백·숫자 등): 현재 음절 마무리 후 그대로.
            cur.flush(&mut out);
            cur = Syl::default();
            out.push(ch);
        }
    }
    cur.flush(&mut out);
    out
}

fn handle_vowel(cur: &mut Syl, jung: usize, out: &mut String) {
    // 받침이 있으면 받침이 다음 글자의 초성으로 넘어간다.
    if cur.cho.is_some() && cur.jung.is_some() && cur.jong != 0 {
        let new_cho = if let Some((first, second_cho)) = jong_split(cur.jong) {
            cur.jong = first;
            second_cho
        } else {
            let cho = jong_to_cho(cur.jong).unwrap_or(11);
            cur.jong = 0;
            cho
        };
        let done = *cur;
        done.flush(out);
        *cur = Syl {
            cho: Some(new_cho),
            jung: Some(jung),
            jong: 0,
        };
        return;
    }
    match (cur.cho, cur.jung) {
        // 초성만 있으면 중성 채우기(CV 완성).
        (Some(_), None) => cur.jung = Some(jung),
        // 이미 중성이 있으면 겹모음 결합, 안 되면 새 음절(초성 없음).
        (_, Some(j)) => {
            if let Some(combined) = vowel_combine(j, jung) {
                cur.jung = Some(combined);
            } else {
                cur.flush(out);
                *cur = Syl {
                    cho: None,
                    jung: Some(jung),
                    jong: 0,
                };
            }
        }
        // 비어 있으면 중성만 있는 음절 시작.
        (None, None) => cur.jung = Some(jung),
    }
}

fn handle_consonant(cur: &mut Syl, cho: usize, jong: Option<usize>, out: &mut String) {
    match (cur.cho, cur.jung, cur.jong) {
        // CVC에서 겹받침 시도, 안 되면 새 음절.
        (Some(_), Some(_), prev) if prev != 0 => match jong.and_then(|cj| jong_combine(prev, cj)) {
            Some(combined) => cur.jong = combined,
            None => {
                cur.flush(out);
                *cur = Syl {
                    cho: Some(cho),
                    jung: None,
                    jong: 0,
                };
            }
        },
        // CV에 받침 가능한 자음이면 받침으로, 아니면 새 음절.
        (Some(_), Some(_), 0) => {
            if let Some(j) = jong {
                cur.jong = j;
            } else {
                cur.flush(out);
                *cur = Syl {
                    cho: Some(cho),
                    jung: None,
                    jong: 0,
                };
            }
        }
        // 초성만/모음만/비어 있음 → 현재 마무리하고 새 초성.
        _ => {
            cur.flush(out);
            *cur = Syl {
                cho: Some(cho),
                jung: None,
                jong: 0,
            };
        }
    }
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

    #[test]
    fn eng_to_han_basic() {
        assert_eq!(eng_to_han("dkssud"), "안녕");
        assert_eq!(eng_to_han("gksrmf"), "한글");
        assert_eq!(eng_to_han("dnjswkd"), "원장");
    }

    #[test]
    fn eng_to_han_compound() {
        assert_eq!(eng_to_han("rhk"), "과"); // 겹모음 ㅘ
        assert_eq!(eng_to_han("ekfr"), "닭"); // 겹받침 ㄺ
        assert_eq!(eng_to_han("rksk"), "가나"); // 받침 이동
    }

    #[test]
    fn eng_to_han_keeps_space_and_digits() {
        assert_eq!(eng_to_han("dkssud gktpdy"), "안녕 하세요");
        assert_eq!(eng_to_han("dks123"), "안123");
    }

    #[test]
    fn round_trip_with_han_to_eng() {
        for word in ["안녕하세요", "원장에이전트", "닭볶음", "과일"] {
            assert_eq!(eng_to_han(&han_to_eng(word)), word, "round trip: {word}");
        }
    }
}
