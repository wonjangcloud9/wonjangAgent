//! 한글 → 로마자(이름 표기). 국립국어원 표기법 기준, 음절별·자음동화 미반영.
//!
//! 성씨는 여권에 흔히 쓰는 관용 표기(Kim/Lee/Park…), 이름은 표준 로마자로 변환한다.
//! 자모 분해(초성·중성·종성)는 유니코드 한글 음절 공식 그대로라 외부 데이터·키가 필요 없다.

const CHO: [&str; 19] = [
    "g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj", "ch", "k", "t", "p",
    "h",
];
const JUNG: [&str; 21] = [
    "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "wo", "we",
    "wi", "yu", "eu", "ui", "i",
];
// 종성 대표음(받침). 0번은 받침 없음.
const JONG: [&str; 28] = [
    "", "k", "k", "k", "n", "n", "n", "t", "l", "k", "m", "l", "l", "l", "p", "l", "m", "p", "p",
    "t", "t", "ng", "t", "t", "k", "t", "p", "t",
];

/// 한글 한 음절을 로마자로(자모 분해). 한글이 아니면 그대로 둔다.
fn syllable(c: char) -> String {
    let o = c as u32;
    if (0xAC00..=0xD7A3).contains(&o) {
        let i = o - 0xAC00;
        let cho = (i / 588) as usize;
        let jung = ((i % 588) / 28) as usize;
        let jong = (i % 28) as usize;
        format!("{}{}{}", CHO[cho], JUNG[jung], JONG[jong])
    } else {
        c.to_string()
    }
}

/// 한글 문자열을 음절별로 로마자화(붙여서). 자음동화는 반영하지 않는다(이름 표기 규칙).
pub fn romanize(s: &str) -> String {
    s.chars().map(syllable).collect()
}

/// 첫 글자만 대문자, 나머지는 소문자.
pub fn capitalize(s: &str) -> String {
    let mut ch = s.chars();
    match ch.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &ch.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// 흔한 한 글자 성씨의 관용 영문 표기. 없으면 None(표준 로마자로 폴백).
pub fn surname(c: char) -> Option<&'static str> {
    Some(match c {
        '김' => "Kim",
        '이' => "Lee",
        '박' => "Park",
        '최' => "Choi",
        '정' => "Jung",
        '강' => "Kang",
        '조' => "Cho",
        '윤' => "Yoon",
        '장' => "Jang",
        '임' => "Lim",
        '한' => "Han",
        '오' => "Oh",
        '서' => "Seo",
        '신' => "Shin",
        '권' => "Kwon",
        '황' => "Hwang",
        '안' => "Ahn",
        '송' => "Song",
        '전' => "Jeon",
        '홍' => "Hong",
        '유' => "Yoo",
        '고' => "Ko",
        '문' => "Moon",
        '양' => "Yang",
        '손' => "Son",
        '배' => "Bae",
        '백' => "Baek",
        '허' => "Heo",
        '심' => "Shim",
        '하' => "Ha",
        '곽' => "Kwak",
        '성' => "Sung",
        '차' => "Cha",
        '주' => "Joo",
        '우' => "Woo",
        '구' => "Koo",
        '민' => "Min",
        '류' => "Ryu",
        '나' => "Na",
        _ => return None,
    })
}

/// 두 글자 복성(흔한 것만).
pub fn is_compound_surname(a: char, b: char) -> bool {
    matches!(
        (a, b),
        ('남', '궁') | ('황', '보') | ('선', '우') | ('제', '갈') | ('사', '공') | ('서', '문')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_decomposition() {
        assert_eq!(romanize("홍길동"), "honggildong");
        assert_eq!(romanize("김민준"), "gimminjun");
        assert_eq!(romanize("이서연"), "iseoyeon");
        assert_eq!(romanize("박지후"), "bakjihu");
        assert_eq!(romanize("철수"), "cheolsu");
    }

    #[test]
    fn capitalize_first() {
        assert_eq!(capitalize("gildong"), "Gildong");
        assert_eq!(capitalize("seoyeon"), "Seoyeon");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn conventional_surnames() {
        assert_eq!(surname('김'), Some("Kim"));
        assert_eq!(surname('이'), Some("Lee"));
        assert_eq!(surname('박'), Some("Park"));
        assert_eq!(surname('홍'), Some("Hong"));
        assert_eq!(surname('갑'), None); // 표에 없는 성 → 폴백
    }
}
