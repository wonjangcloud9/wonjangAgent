//! "오늘 뭐 먹지?" 메뉴 추천 — 매일 식사 고민을 덜어주는 비서 기능.
//!
//! 카테고리별로 한국에서 흔히 먹는 메뉴를 모아두고, 시간 기반 시드로
//! 무작위 추천한다. 외부 의존성·API 키가 필요 없다.

use std::time::{SystemTime, UNIX_EPOCH};

/// 메뉴 카테고리(한국어 별칭과 메뉴 목록).
pub struct Category {
    pub key: &'static str,
    pub menus: &'static [&'static str],
}

const CATEGORIES: &[Category] = &[
    Category {
        key: "한식",
        menus: &[
            "김치찌개",
            "된장찌개",
            "비빔밥",
            "제육볶음",
            "불고기",
            "삼겹살",
            "순두부찌개",
            "김치볶음밥",
            "갈비탕",
            "냉면",
            "보쌈",
            "닭갈비",
            "감자탕",
            "부대찌개",
        ],
    },
    Category {
        key: "중식",
        menus: &[
            "짜장면",
            "짬뽕",
            "탕수육",
            "마라탕",
            "볶음밥",
            "유린기",
            "마파두부",
            "깐풍기",
        ],
    },
    Category {
        key: "일식",
        menus: &[
            "초밥",
            "라멘",
            "돈카츠",
            "규동",
            "우동",
            "가츠동",
            "소바",
            "오므라이스",
        ],
    },
    Category {
        key: "양식",
        menus: &[
            "파스타",
            "피자",
            "스테이크",
            "리조또",
            "햄버거",
            "샐러드",
            "그라탱",
        ],
    },
    Category {
        key: "분식",
        menus: &[
            "떡볶이",
            "김밥",
            "라면",
            "순대",
            "튀김",
            "쫄면",
            "만두",
            "토스트",
        ],
    },
    Category {
        key: "야식",
        menus: &[
            "치킨",
            "족발",
            "보쌈",
            "곱창",
            "닭발",
            "피자",
            "마라탕",
            "야식김밥",
        ],
    },
];

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// 별칭으로 카테고리를 찾는다(없으면 None).
pub fn find_category(name: &str) -> Option<&'static Category> {
    CATEGORIES.iter().find(|c| c.key == name)
}

/// 등록된 카테고리 키 목록.
pub fn category_keys() -> Vec<&'static str> {
    CATEGORIES.iter().map(|c| c.key).collect()
}

/// 시드로 메뉴 하나를 고른다. `category`가 None이면 전체에서 고른다.
/// 반환은 (카테고리, 메뉴).
pub fn pick(category: Option<&str>, seed: u64) -> Option<(&'static str, &'static str)> {
    let mut state = seed | 1;
    let pool: Vec<(&'static str, &'static str)> = match category {
        Some(name) => {
            let cat = find_category(name)?;
            cat.menus.iter().map(|m| (cat.key, *m)).collect()
        }
        None => CATEGORIES
            .iter()
            .flat_map(|c| c.menus.iter().map(move |m| (c.key, *m)))
            .collect(),
    };
    if pool.is_empty() {
        return None;
    }
    let idx = (xorshift(&mut state) % pool.len() as u64) as usize;
    Some(pool[idx])
}

/// 시간 기반 시드로 즉석 추천.
pub fn recommend(category: Option<&str>) -> Option<(&'static str, &'static str)> {
    pick(category, now_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_from_known_category() {
        let (cat, menu) = pick(Some("중식"), 12345).unwrap();
        assert_eq!(cat, "중식");
        assert!(find_category("중식").unwrap().menus.contains(&menu));
    }

    #[test]
    fn pick_from_all_is_some() {
        assert!(pick(None, 999).is_some());
    }

    #[test]
    fn unknown_category_is_none() {
        assert!(pick(Some("우주식"), 1).is_none());
    }

    #[test]
    fn deterministic_with_same_seed() {
        assert_eq!(pick(None, 42), pick(None, 42));
    }
}
